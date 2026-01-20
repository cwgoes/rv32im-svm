//! rv32im to SVM translator
//!
//! This module translates RISC-V rv32im instructions to SVM bytecode.
//!
//! Memory layout:
//! - 0x0000 - 0x7FFF: Program data/heap
//! - 0x8000 - 0x807F: RISC-V register file (32 * 4 bytes = 128 bytes)
//! - 0x8080+: Stack (grows downward)
//!
//! The compiler uses a memory-based register file approach where all RISC-V
//! registers are stored in memory. SVM registers are used as temporaries.

use crate::riscv::{Instruction, Register};
use crate::svm::{opcodes, MemorySize, SvmInstruction, SvmProgram, SvmRegister};
use std::collections::HashMap;
use thiserror::Error;

/// Compiler error
#[derive(Debug, Error)]
pub enum CompilerError {
    #[error("Unsupported instruction: {0}")]
    UnsupportedInstruction(String),

    #[error("Invalid branch target: {0}")]
    InvalidBranchTarget(i32),

    #[error("Program too large: {0} instructions")]
    ProgramTooLarge(usize),
}

/// Compiler result type
pub type CompilerResult<T> = Result<T, CompilerError>;

/// Compiler configuration
#[derive(Debug, Clone)]
pub struct CompilerConfig {
    /// Base address for RISC-V register file in SVM memory
    pub regfile_base: u64,

    /// Base address for RISC-V memory in SVM memory
    pub memory_base: u64,

    /// Stack base address
    pub stack_base: u64,

    /// Initial PC value (byte address in RISC-V)
    pub initial_pc: u32,
}

impl Default for CompilerConfig {
    fn default() -> Self {
        CompilerConfig {
            regfile_base: 0x8000,
            memory_base: 0x0000,
            stack_base: 0x10000, // 64KB
            initial_pc: 0,
        }
    }
}

/// rv32im to SVM compiler
pub struct Compiler {
    config: CompilerConfig,
    /// Output SVM program
    output: SvmProgram,
    /// Mapping from RISC-V instruction index to SVM instruction index
    pc_map: HashMap<usize, usize>,
    /// RISC-V instructions being compiled
    rv_instructions: Vec<Instruction>,
    /// Current RISC-V instruction index
    rv_pc: usize,
    /// Pending branch fixups: (svm_index, rv_target_index)
    branch_fixups: Vec<(usize, usize)>,
    /// Return sites: RV byte addresses that can be return targets (after JAL)
    return_sites: Vec<usize>,
    /// JALR dispatch fixups: (svm_index) - jump instructions that need dispatch table target
    jalr_fixups: Vec<usize>,
}

impl Compiler {
    /// Create a new compiler with default configuration
    pub fn new() -> Self {
        Self::with_config(CompilerConfig::default())
    }

    /// Create a new compiler with given configuration
    pub fn with_config(config: CompilerConfig) -> Self {
        Compiler {
            config,
            output: SvmProgram::new(),
            pc_map: HashMap::new(),
            rv_instructions: Vec::new(),
            rv_pc: 0,
            branch_fixups: Vec::new(),
            return_sites: Vec::new(),
            jalr_fixups: Vec::new(),
        }
    }

    /// Compile a sequence of RISC-V instructions
    pub fn compile(&mut self, instructions: &[Instruction]) -> CompilerResult<SvmProgram> {
        self.rv_instructions = instructions.to_vec();
        self.output = SvmProgram::new();
        self.pc_map.clear();
        self.branch_fixups.clear();
        self.return_sites.clear();
        self.jalr_fixups.clear();

        // First pass: emit prologue
        self.emit_prologue();

        // Second pass: compile each instruction
        for (idx, inst) in instructions.iter().enumerate() {
            self.rv_pc = idx;
            // Record PC mapping before emitting
            self.pc_map.insert(idx, self.output.len());
            self.compile_instruction(*inst)?;
        }

        // Record the "end" position for branches that jump past the last instruction
        self.pc_map.insert(instructions.len(), self.output.len());

        // Third pass: fix up branches
        self.fixup_branches()?;

        // Fourth pass: emit return dispatch table if needed
        self.emit_return_dispatch()?;

        // Emit epilogue (return)
        self.emit_epilogue();

        Ok(std::mem::take(&mut self.output))
    }

    /// Compile RISC-V binary (array of 32-bit words)
    pub fn compile_binary(&mut self, binary: &[u32]) -> CompilerResult<SvmProgram> {
        let instructions: Vec<Instruction> = binary
            .iter()
            .map(|&word| crate::riscv::decode(word))
            .collect();
        self.compile(&instructions)
    }

    fn emit_prologue(&mut self) {
        // Set up SVM r10 (stack pointer) - it's read-only so we use r9 as our stack pointer
        // Initialize r9 to point to top of stack
        let stack_top = self.config.stack_base as i32;
        self.output.push(SvmInstruction::mov64_imm(SvmRegister::R9, stack_top));

        // Initialize RISC-V x2 (sp) in register file
        let sp_addr = self.regfile_addr(Register::SP);
        self.output.push(SvmInstruction::mov64_imm(SvmRegister::R1, sp_addr as i32));
        self.output.push(SvmInstruction::st(MemorySize::Word, SvmRegister::R1, 0, stack_top));
    }

    fn emit_epilogue(&mut self) {
        // Load return value from a0 (x10) to r0
        self.load_rv_reg(SvmRegister::R0, Register::A0);
        self.output.push(SvmInstruction::exit());
    }

    /// Get memory address for a RISC-V register
    fn regfile_addr(&self, reg: Register) -> u64 {
        self.config.regfile_base + (reg.index() as u64 * 4)
    }

    /// Load a RISC-V register value into an SVM register
    fn load_rv_reg(&mut self, svm_reg: SvmRegister, rv_reg: Register) {
        if rv_reg == Register::ZERO {
            // x0 is always 0
            self.output.push(SvmInstruction::mov64_imm(svm_reg, 0));
        } else {
            let addr = self.regfile_addr(rv_reg);
            self.output.push(SvmInstruction::mov64_imm(SvmRegister::R8, addr as i32));
            self.output.push(SvmInstruction::ldx(MemorySize::Word, svm_reg, SvmRegister::R8, 0));
            // Sign extend from 32-bit to 64-bit
            self.output.push(SvmInstruction::lsh64_imm(svm_reg, 32));
            self.output.push(SvmInstruction::arsh64_imm(svm_reg, 32));
        }
    }

    /// Store an SVM register value to a RISC-V register
    fn store_rv_reg(&mut self, rv_reg: Register, svm_reg: SvmRegister) {
        if rv_reg == Register::ZERO {
            // Writes to x0 are ignored
            return;
        }
        let addr = self.regfile_addr(rv_reg);
        self.output.push(SvmInstruction::mov64_imm(SvmRegister::R8, addr as i32));
        self.output.push(SvmInstruction::stx(MemorySize::Word, SvmRegister::R8, svm_reg, 0));
    }

    /// Compile a single RISC-V instruction
    fn compile_instruction(&mut self, inst: Instruction) -> CompilerResult<()> {
        use Instruction::*;

        match inst {
            // R-type ALU operations
            Add { rd, rs1, rs2 } => self.compile_r_type_svm(rd, rs1, rs2, opcodes::ADD),
            Sub { rd, rs1, rs2 } => self.compile_r_type_svm(rd, rs1, rs2, opcodes::SUB),
            Xor { rd, rs1, rs2 } => self.compile_r_type_svm(rd, rs1, rs2, opcodes::XOR),
            Or { rd, rs1, rs2 } => self.compile_r_type_svm(rd, rs1, rs2, opcodes::OR),
            And { rd, rs1, rs2 } => self.compile_r_type_svm(rd, rs1, rs2, opcodes::AND),
            Sll { rd, rs1, rs2 } => self.compile_shift(rd, rs1, rs2, false, false),
            Srl { rd, rs1, rs2 } => self.compile_shift(rd, rs1, rs2, true, false),
            Sra { rd, rs1, rs2 } => self.compile_shift(rd, rs1, rs2, true, true),
            Slt { rd, rs1, rs2 } => self.compile_slt(rd, rs1, rs2, true),
            Sltu { rd, rs1, rs2 } => self.compile_slt(rd, rs1, rs2, false),

            // I-type ALU operations
            Addi { rd, rs1, imm } => self.compile_i_type_add(rd, rs1, imm),
            Xori { rd, rs1, imm } => self.compile_i_type(rd, rs1, imm, opcodes::XOR),
            Ori { rd, rs1, imm } => self.compile_i_type(rd, rs1, imm, opcodes::OR),
            Andi { rd, rs1, imm } => self.compile_i_type(rd, rs1, imm, opcodes::AND),
            Slli { rd, rs1, shamt } => self.compile_shift_imm(rd, rs1, shamt, false, false),
            Srli { rd, rs1, shamt } => self.compile_shift_imm(rd, rs1, shamt, true, false),
            Srai { rd, rs1, shamt } => self.compile_shift_imm(rd, rs1, shamt, true, true),
            Slti { rd, rs1, imm } => self.compile_slti(rd, rs1, imm, true),
            Sltiu { rd, rs1, imm } => self.compile_slti(rd, rs1, imm, false),

            // Load instructions
            Lb { rd, rs1, imm } => self.compile_load(rd, rs1, imm, MemorySize::Byte, true),
            Lh { rd, rs1, imm } => self.compile_load(rd, rs1, imm, MemorySize::Half, true),
            Lw { rd, rs1, imm } => self.compile_load(rd, rs1, imm, MemorySize::Word, true),
            Lbu { rd, rs1, imm } => self.compile_load(rd, rs1, imm, MemorySize::Byte, false),
            Lhu { rd, rs1, imm } => self.compile_load(rd, rs1, imm, MemorySize::Half, false),

            // Store instructions
            Sb { rs1, rs2, imm } => self.compile_store(rs1, rs2, imm, MemorySize::Byte),
            Sh { rs1, rs2, imm } => self.compile_store(rs1, rs2, imm, MemorySize::Half),
            Sw { rs1, rs2, imm } => self.compile_store(rs1, rs2, imm, MemorySize::Word),

            // Branch instructions
            Beq { rs1, rs2, imm } => self.compile_branch(rs1, rs2, imm, opcodes::JEQ),
            Bne { rs1, rs2, imm } => self.compile_branch(rs1, rs2, imm, opcodes::JNE),
            Blt { rs1, rs2, imm } => self.compile_branch(rs1, rs2, imm, opcodes::JSLT),
            Bge { rs1, rs2, imm } => self.compile_branch(rs1, rs2, imm, opcodes::JSGE),
            Bltu { rs1, rs2, imm } => self.compile_branch(rs1, rs2, imm, opcodes::JLT),
            Bgeu { rs1, rs2, imm } => self.compile_branch(rs1, rs2, imm, opcodes::JGE),

            // Jump instructions
            Jal { rd, imm } => self.compile_jal(rd, imm),
            Jalr { rd, rs1, imm } => self.compile_jalr(rd, rs1, imm),

            // U-type instructions
            Lui { rd, imm } => self.compile_lui(rd, imm),
            Auipc { rd, imm } => self.compile_auipc(rd, imm),

            // System instructions
            Ecall => self.compile_ecall(),
            Ebreak => self.compile_ebreak(),
            Fence { .. } | FenceI => {
                // Fence instructions are no-ops in our single-threaded model
                Ok(())
            }

            // CSR instructions - simplified handling
            Csrrw { rd, rs1, csr } => self.compile_csr(rd, rs1, csr),
            Csrrs { rd, rs1, csr } => self.compile_csr(rd, rs1, csr),
            Csrrc { rd, rs1, csr } => self.compile_csr(rd, rs1, csr),
            Csrrwi { rd, uimm: _, csr } => self.compile_csr_imm(rd, csr),
            Csrrsi { rd, uimm: _, csr } => self.compile_csr_imm(rd, csr),
            Csrrci { rd, uimm: _, csr } => self.compile_csr_imm(rd, csr),

            // M extension
            Mul { rd, rs1, rs2 } => self.compile_mul(rd, rs1, rs2),
            Mulh { rd, rs1, rs2 } => self.compile_mulh(rd, rs1, rs2, true, true),
            Mulhsu { rd, rs1, rs2 } => self.compile_mulh(rd, rs1, rs2, true, false),
            Mulhu { rd, rs1, rs2 } => self.compile_mulh(rd, rs1, rs2, false, false),
            Div { rd, rs1, rs2 } => self.compile_div(rd, rs1, rs2, true),
            Divu { rd, rs1, rs2 } => self.compile_div(rd, rs1, rs2, false),
            Rem { rd, rs1, rs2 } => self.compile_rem(rd, rs1, rs2, true),
            Remu { rd, rs1, rs2 } => self.compile_rem(rd, rs1, rs2, false),

            Unknown(word) => {
                Err(CompilerError::UnsupportedInstruction(format!("{:#010x}", word)))
            }
        }
    }

    // R-type using SVM opcodes directly
    fn compile_r_type_svm(&mut self, rd: Register, rs1: Register, rs2: Register, op: u8) -> CompilerResult<()> {
        self.load_rv_reg(SvmRegister::R1, rs1);
        self.load_rv_reg(SvmRegister::R2, rs2);
        self.output.push(SvmInstruction::alu32_reg(op, SvmRegister::R1, SvmRegister::R2));
        // Sign extend result
        self.output.push(SvmInstruction::lsh64_imm(SvmRegister::R1, 32));
        self.output.push(SvmInstruction::arsh64_imm(SvmRegister::R1, 32));
        self.store_rv_reg(rd, SvmRegister::R1);
        Ok(())
    }

    // I-type ALU operations
    fn compile_i_type(&mut self, rd: Register, rs1: Register, imm: i32, op: u8) -> CompilerResult<()> {
        self.load_rv_reg(SvmRegister::R1, rs1);
        self.output.push(SvmInstruction::alu32_imm(op, SvmRegister::R1, imm));
        // Sign extend result
        self.output.push(SvmInstruction::lsh64_imm(SvmRegister::R1, 32));
        self.output.push(SvmInstruction::arsh64_imm(SvmRegister::R1, 32));
        self.store_rv_reg(rd, SvmRegister::R1);
        Ok(())
    }

    // Special handling for ADDI (used for many pseudo-instructions)
    fn compile_i_type_add(&mut self, rd: Register, rs1: Register, imm: i32) -> CompilerResult<()> {
        self.load_rv_reg(SvmRegister::R1, rs1);
        self.output.push(SvmInstruction::add32_imm(SvmRegister::R1, imm));
        // Sign extend result
        self.output.push(SvmInstruction::lsh64_imm(SvmRegister::R1, 32));
        self.output.push(SvmInstruction::arsh64_imm(SvmRegister::R1, 32));
        self.store_rv_reg(rd, SvmRegister::R1);
        Ok(())
    }

    // Shift operations with register
    fn compile_shift(&mut self, rd: Register, rs1: Register, rs2: Register, right: bool, arithmetic: bool) -> CompilerResult<()> {
        self.load_rv_reg(SvmRegister::R1, rs1);
        self.load_rv_reg(SvmRegister::R2, rs2);
        // Mask shift amount to 5 bits
        self.output.push(SvmInstruction::and32_imm(SvmRegister::R2, 0x1f));

        if right {
            if arithmetic {
                // For arithmetic right shift, first sign extend to 64-bit, shift, then truncate
                self.output.push(SvmInstruction::lsh64_imm(SvmRegister::R1, 32));
                self.output.push(SvmInstruction::arsh64_imm(SvmRegister::R1, 32));
                self.output.push(SvmInstruction::arsh64_reg(SvmRegister::R1, SvmRegister::R2));
            } else {
                // Logical right shift - zero extend first by shifting left then right
                self.output.push(SvmInstruction::lsh64_imm(SvmRegister::R1, 32));
                self.output.push(SvmInstruction::rsh64_imm(SvmRegister::R1, 32));
                self.output.push(SvmInstruction::rsh64_reg(SvmRegister::R1, SvmRegister::R2));
            }
        } else {
            self.output.push(SvmInstruction::lsh64_reg(SvmRegister::R1, SvmRegister::R2));
        }

        // Truncate to 32-bit and sign extend
        self.output.push(SvmInstruction::lsh64_imm(SvmRegister::R1, 32));
        self.output.push(SvmInstruction::arsh64_imm(SvmRegister::R1, 32));
        self.store_rv_reg(rd, SvmRegister::R1);
        Ok(())
    }

    // Shift operations with immediate
    fn compile_shift_imm(&mut self, rd: Register, rs1: Register, shamt: u32, right: bool, arithmetic: bool) -> CompilerResult<()> {
        self.load_rv_reg(SvmRegister::R1, rs1);
        let shamt = (shamt & 0x1f) as i32;

        if right {
            if arithmetic {
                // Sign extend first, then arithmetic shift
                self.output.push(SvmInstruction::lsh64_imm(SvmRegister::R1, 32));
                self.output.push(SvmInstruction::arsh64_imm(SvmRegister::R1, 32));
                self.output.push(SvmInstruction::arsh64_imm(SvmRegister::R1, shamt));
            } else {
                // Zero extend (lsh then rsh) and logical shift
                self.output.push(SvmInstruction::lsh64_imm(SvmRegister::R1, 32));
                self.output.push(SvmInstruction::rsh64_imm(SvmRegister::R1, 32));
                self.output.push(SvmInstruction::rsh64_imm(SvmRegister::R1, shamt));
            }
        } else {
            self.output.push(SvmInstruction::lsh64_imm(SvmRegister::R1, shamt));
        }

        // Truncate to 32-bit and sign extend
        self.output.push(SvmInstruction::lsh64_imm(SvmRegister::R1, 32));
        self.output.push(SvmInstruction::arsh64_imm(SvmRegister::R1, 32));
        self.store_rv_reg(rd, SvmRegister::R1);
        Ok(())
    }

    // Set less than (register)
    fn compile_slt(&mut self, rd: Register, rs1: Register, rs2: Register, signed: bool) -> CompilerResult<()> {
        self.load_rv_reg(SvmRegister::R1, rs1);
        self.load_rv_reg(SvmRegister::R2, rs2);

        // Use conditional jump to set result
        // r3 = 1 (assume less than)
        self.output.push(SvmInstruction::mov64_imm(SvmRegister::R3, 1));

        if signed {
            // jslt r1, r2, +1
            self.output.push(SvmInstruction::jmp_reg(opcodes::JSLT, SvmRegister::R1, SvmRegister::R2, 1));
        } else {
            // jlt r1, r2, +1
            self.output.push(SvmInstruction::jmp_reg(opcodes::JLT, SvmRegister::R1, SvmRegister::R2, 1));
        }

        // r3 = 0 (not less than)
        self.output.push(SvmInstruction::mov64_imm(SvmRegister::R3, 0));

        self.store_rv_reg(rd, SvmRegister::R3);
        Ok(())
    }

    // Set less than immediate
    fn compile_slti(&mut self, rd: Register, rs1: Register, imm: i32, signed: bool) -> CompilerResult<()> {
        self.load_rv_reg(SvmRegister::R1, rs1);

        // r3 = 1 (assume less than)
        self.output.push(SvmInstruction::mov64_imm(SvmRegister::R3, 1));

        if signed {
            self.output.push(SvmInstruction::jmp_imm(opcodes::JSLT, SvmRegister::R1, imm, 1));
        } else {
            // For unsigned, we need to handle sign extension carefully
            self.output.push(SvmInstruction::jmp_imm(opcodes::JLT, SvmRegister::R1, imm, 1));
        }

        self.output.push(SvmInstruction::mov64_imm(SvmRegister::R3, 0));

        self.store_rv_reg(rd, SvmRegister::R3);
        Ok(())
    }

    // Load instructions
    fn compile_load(&mut self, rd: Register, rs1: Register, imm: i32, size: MemorySize, sign_extend: bool) -> CompilerResult<()> {
        // Calculate address: base + imm + memory_base
        self.load_rv_reg(SvmRegister::R1, rs1);
        self.output.push(SvmInstruction::add64_imm(SvmRegister::R1, imm));
        self.output.push(SvmInstruction::add64_imm(SvmRegister::R1, self.config.memory_base as i32));

        // Load value
        self.output.push(SvmInstruction::ldx(size, SvmRegister::R2, SvmRegister::R1, 0));

        // Sign or zero extend to 32 bits
        if sign_extend {
            match size {
                MemorySize::Byte => {
                    self.output.push(SvmInstruction::lsh64_imm(SvmRegister::R2, 56));
                    self.output.push(SvmInstruction::arsh64_imm(SvmRegister::R2, 56));
                }
                MemorySize::Half => {
                    self.output.push(SvmInstruction::lsh64_imm(SvmRegister::R2, 48));
                    self.output.push(SvmInstruction::arsh64_imm(SvmRegister::R2, 48));
                }
                MemorySize::Word => {
                    self.output.push(SvmInstruction::lsh64_imm(SvmRegister::R2, 32));
                    self.output.push(SvmInstruction::arsh64_imm(SvmRegister::R2, 32));
                }
                MemorySize::DWord => {}
            }
        }

        self.store_rv_reg(rd, SvmRegister::R2);
        Ok(())
    }

    // Store instructions
    fn compile_store(&mut self, rs1: Register, rs2: Register, imm: i32, size: MemorySize) -> CompilerResult<()> {
        // Calculate address
        self.load_rv_reg(SvmRegister::R1, rs1);
        self.output.push(SvmInstruction::add64_imm(SvmRegister::R1, imm));
        self.output.push(SvmInstruction::add64_imm(SvmRegister::R1, self.config.memory_base as i32));

        // Load value to store
        self.load_rv_reg(SvmRegister::R2, rs2);

        // Store
        self.output.push(SvmInstruction::stx(size, SvmRegister::R1, SvmRegister::R2, 0));
        Ok(())
    }

    // Branch instructions
    fn compile_branch(&mut self, rs1: Register, rs2: Register, imm: i32, jmp_op: u8) -> CompilerResult<()> {
        self.load_rv_reg(SvmRegister::R1, rs1);
        self.load_rv_reg(SvmRegister::R2, rs2);

        // Calculate target instruction index
        // imm is byte offset, divide by 4 to get instruction offset
        let inst_offset = imm / 4;
        let target_idx = (self.rv_pc as i32 + inst_offset) as usize;

        // Emit jump with placeholder offset (will be fixed up later)
        let jmp_idx = self.output.len();
        self.output.push(SvmInstruction::jmp_reg(jmp_op, SvmRegister::R1, SvmRegister::R2, 0));

        // Record fixup
        self.branch_fixups.push((jmp_idx, target_idx));
        Ok(())
    }

    // JAL instruction
    fn compile_jal(&mut self, rd: Register, imm: i32) -> CompilerResult<()> {
        // Save return address (next instruction's address in RISC-V terms)
        let return_addr = (self.rv_pc + 1) * 4;
        self.output.push(SvmInstruction::mov64_imm(SvmRegister::R1, return_addr as i32));
        self.store_rv_reg(rd, SvmRegister::R1);

        // Record this as a potential return site (for JALR dispatch)
        if rd != Register::ZERO {
            // Only record if we're saving the return address
            if !self.return_sites.contains(&(self.rv_pc + 1)) {
                self.return_sites.push(self.rv_pc + 1);
            }
        }

        // Calculate target
        let inst_offset = imm / 4;
        let target_idx = (self.rv_pc as i32 + inst_offset) as usize;

        // Emit unconditional jump
        let jmp_idx = self.output.len();
        self.output.push(SvmInstruction::ja(0));

        self.branch_fixups.push((jmp_idx, target_idx));
        Ok(())
    }

    // JALR instruction
    fn compile_jalr(&mut self, rd: Register, rs1: Register, imm: i32) -> CompilerResult<()> {
        // Calculate target address in bytes
        self.load_rv_reg(SvmRegister::R1, rs1);
        self.output.push(SvmInstruction::add64_imm(SvmRegister::R1, imm));
        // Clear lowest bit (per RISC-V spec)
        self.output.push(SvmInstruction::and64_imm(SvmRegister::R1, !1));

        // Save return address (for calls, not returns)
        if rd != Register::ZERO {
            let return_addr = ((self.rv_pc + 1) * 4) as i32;
            self.output.push(SvmInstruction::mov64_imm(SvmRegister::R2, return_addr));
            self.store_rv_reg(rd, SvmRegister::R2);
        }

        // R1 now contains the target byte address
        // We'll jump to the dispatch table which will handle the dynamic jump
        // Store jump index for later fixup
        let jmp_idx = self.output.len();
        self.output.push(SvmInstruction::ja(0)); // Will be fixed up to point to dispatch table
        self.jalr_fixups.push(jmp_idx);

        Ok(())
    }

    // LUI instruction
    fn compile_lui(&mut self, rd: Register, imm: i32) -> CompilerResult<()> {
        // imm already has lower 12 bits as 0
        self.output.push(SvmInstruction::mov64_imm(SvmRegister::R1, imm));
        self.store_rv_reg(rd, SvmRegister::R1);
        Ok(())
    }

    // AUIPC instruction
    fn compile_auipc(&mut self, rd: Register, imm: i32) -> CompilerResult<()> {
        // PC + imm (imm is upper 20 bits << 12)
        let pc_val = (self.rv_pc * 4) as i32;
        let result = pc_val.wrapping_add(imm);
        self.output.push(SvmInstruction::mov64_imm(SvmRegister::R1, result));
        self.store_rv_reg(rd, SvmRegister::R1);
        Ok(())
    }

    // ECALL - system call
    fn compile_ecall(&mut self) -> CompilerResult<()> {
        // In our model, ecall with a7=93 is exit
        // Load return value from a0 and exit
        self.load_rv_reg(SvmRegister::R0, Register::A0);
        self.output.push(SvmInstruction::exit());
        Ok(())
    }

    // EBREAK
    fn compile_ebreak(&mut self) -> CompilerResult<()> {
        // For now, treat as exit
        self.output.push(SvmInstruction::exit());
        Ok(())
    }

    // CSR instructions (simplified - most CSRs are ignored)
    fn compile_csr(&mut self, rd: Register, _rs1: Register, _csr: u16) -> CompilerResult<()> {
        // Return 0 for most CSRs
        self.output.push(SvmInstruction::mov64_imm(SvmRegister::R1, 0));
        self.store_rv_reg(rd, SvmRegister::R1);
        Ok(())
    }

    fn compile_csr_imm(&mut self, rd: Register, _csr: u16) -> CompilerResult<()> {
        self.output.push(SvmInstruction::mov64_imm(SvmRegister::R1, 0));
        self.store_rv_reg(rd, SvmRegister::R1);
        Ok(())
    }

    // MUL instruction
    fn compile_mul(&mut self, rd: Register, rs1: Register, rs2: Register) -> CompilerResult<()> {
        self.load_rv_reg(SvmRegister::R1, rs1);
        self.load_rv_reg(SvmRegister::R2, rs2);
        self.output.push(SvmInstruction::mul32_reg(SvmRegister::R1, SvmRegister::R2));
        // Sign extend
        self.output.push(SvmInstruction::lsh64_imm(SvmRegister::R1, 32));
        self.output.push(SvmInstruction::arsh64_imm(SvmRegister::R1, 32));
        self.store_rv_reg(rd, SvmRegister::R1);
        Ok(())
    }

    // MULH, MULHSU, MULHU - get upper 32 bits of multiplication
    fn compile_mulh(&mut self, rd: Register, rs1: Register, rs2: Register, rs1_signed: bool, rs2_signed: bool) -> CompilerResult<()> {
        self.load_rv_reg(SvmRegister::R1, rs1);
        self.load_rv_reg(SvmRegister::R2, rs2);

        // Sign/zero extend based on signedness
        if rs1_signed {
            // Already sign-extended by load_rv_reg
        } else {
            // Zero-extend: shift left 32, then logical shift right 32
            self.output.push(SvmInstruction::lsh64_imm(SvmRegister::R1, 32));
            self.output.push(SvmInstruction::rsh64_imm(SvmRegister::R1, 32));
        }

        if rs2_signed {
            // Already sign-extended by load_rv_reg
        } else {
            // Zero-extend
            self.output.push(SvmInstruction::lsh64_imm(SvmRegister::R2, 32));
            self.output.push(SvmInstruction::rsh64_imm(SvmRegister::R2, 32));
        }

        // 64-bit multiply
        self.output.push(SvmInstruction::mul64_reg(SvmRegister::R1, SvmRegister::R2));

        // Get upper 32 bits
        self.output.push(SvmInstruction::rsh64_imm(SvmRegister::R1, 32));
        // Sign extend result for storage
        self.output.push(SvmInstruction::lsh64_imm(SvmRegister::R1, 32));
        self.output.push(SvmInstruction::arsh64_imm(SvmRegister::R1, 32));

        self.store_rv_reg(rd, SvmRegister::R1);
        Ok(())
    }

    // DIV/DIVU instruction
    fn compile_div(&mut self, rd: Register, rs1: Register, rs2: Register, signed: bool) -> CompilerResult<()> {
        self.load_rv_reg(SvmRegister::R1, rs1);
        self.load_rv_reg(SvmRegister::R2, rs2);

        // Check for division by zero - return -1 for signed, 0xFFFFFFFF for unsigned
        self.output.push(SvmInstruction::jmp_imm(opcodes::JNE, SvmRegister::R2, 0, 2));
        self.output.push(SvmInstruction::mov64_imm(SvmRegister::R1, -1));
        let skip_idx = self.output.len();
        self.output.push(SvmInstruction::ja(0)); // Will be fixed up

        if signed {
            // Check for overflow: -2^31 / -1
            self.output.push(SvmInstruction::mov64_imm(SvmRegister::R3, i32::MIN));
            self.output.push(SvmInstruction::jmp_reg(opcodes::JNE, SvmRegister::R1, SvmRegister::R3, 4));
            self.output.push(SvmInstruction::mov64_imm(SvmRegister::R3, -1));
            self.output.push(SvmInstruction::jmp_reg(opcodes::JNE, SvmRegister::R2, SvmRegister::R3, 2));
            // Overflow case: return -2^31
            self.output.push(SvmInstruction::mov64_imm(SvmRegister::R1, i32::MIN));
            let skip_div_idx = self.output.len();
            self.output.push(SvmInstruction::ja(0));

            // Normal signed division
            self.output.push(SvmInstruction::sdiv32_reg(SvmRegister::R1, SvmRegister::R2));

            // Fix up the skip jump
            let current = self.output.len();
            self.output.instructions[skip_div_idx].offset = (current - skip_div_idx - 1) as i16;
        } else {
            // Unsigned division
            self.output.push(SvmInstruction::div32_reg(SvmRegister::R1, SvmRegister::R2));
        }

        // Fix up the division by zero skip
        let current = self.output.len();
        self.output.instructions[skip_idx].offset = (current - skip_idx - 1) as i16;

        // Sign extend result
        self.output.push(SvmInstruction::lsh64_imm(SvmRegister::R1, 32));
        self.output.push(SvmInstruction::arsh64_imm(SvmRegister::R1, 32));

        self.store_rv_reg(rd, SvmRegister::R1);
        Ok(())
    }

    // REM/REMU instruction
    fn compile_rem(&mut self, rd: Register, rs1: Register, rs2: Register, signed: bool) -> CompilerResult<()> {
        self.load_rv_reg(SvmRegister::R1, rs1);
        self.load_rv_reg(SvmRegister::R2, rs2);

        // Check for division by zero - return dividend
        // If R2 != 0, skip 1 instruction (the ja that jumps to end)
        self.output.push(SvmInstruction::jmp_imm(opcodes::JNE, SvmRegister::R2, 0, 1));
        let skip_to_end_idx = self.output.len();
        self.output.push(SvmInstruction::ja(0)); // Skip to end (result is already in R1)

        if signed {
            // Check for overflow: -2^31 % -1 = 0
            // First check if R1 == i32::MIN
            self.output.push(SvmInstruction::mov64_imm(SvmRegister::R3, i32::MIN));
            // If R1 != MIN, skip the overflow check (skip next 4 instructions)
            self.output.push(SvmInstruction::jmp_reg(opcodes::JNE, SvmRegister::R1, SvmRegister::R3, 4));
            // R1 == MIN, now check if R2 == -1
            self.output.push(SvmInstruction::mov64_imm(SvmRegister::R3, -1));
            // If R2 != -1, skip the overflow result (skip next 2 instructions)
            self.output.push(SvmInstruction::jmp_reg(opcodes::JNE, SvmRegister::R2, SvmRegister::R3, 2));
            // Overflow case: return 0
            self.output.push(SvmInstruction::mov64_imm(SvmRegister::R1, 0));
            let skip_rem_idx = self.output.len();
            self.output.push(SvmInstruction::ja(0)); // Jump to end

            // Normal signed remainder using quotient * divisor subtraction method
            // rem = dividend - (dividend / divisor) * divisor

            // Save dividend in R3
            self.output.push(SvmInstruction::mov64_reg(SvmRegister::R3, SvmRegister::R1));

            // Compute signed division: R4 = R1 / R2
            self.output.push(SvmInstruction::sdiv32_reg(SvmRegister::R1, SvmRegister::R2));

            // Compute quotient * divisor: R1 = R1 * R2
            self.output.push(SvmInstruction::mul32_reg(SvmRegister::R1, SvmRegister::R2));

            // Compute remainder: R1 = R3 - R1 (dividend - quotient * divisor)
            self.output.push(SvmInstruction::sub32_reg(SvmRegister::R3, SvmRegister::R1));
            self.output.push(SvmInstruction::mov64_reg(SvmRegister::R1, SvmRegister::R3));

            // Fix up skip jump
            let current = self.output.len();
            self.output.instructions[skip_rem_idx].offset = (current - skip_rem_idx - 1) as i16;
        } else {
            // Unsigned modulo
            // Zero-extend operands first
            self.output.push(SvmInstruction::lsh64_imm(SvmRegister::R1, 32));
            self.output.push(SvmInstruction::rsh64_imm(SvmRegister::R1, 32));
            self.output.push(SvmInstruction::lsh64_imm(SvmRegister::R2, 32));
            self.output.push(SvmInstruction::rsh64_imm(SvmRegister::R2, 32));
            self.output.push(SvmInstruction::mod64_reg(SvmRegister::R1, SvmRegister::R2));
        }

        // Fix up division by zero skip
        let current = self.output.len();
        self.output.instructions[skip_to_end_idx].offset = (current - skip_to_end_idx - 1) as i16;

        // Sign extend result
        self.output.push(SvmInstruction::lsh64_imm(SvmRegister::R1, 32));
        self.output.push(SvmInstruction::arsh64_imm(SvmRegister::R1, 32));

        self.store_rv_reg(rd, SvmRegister::R1);
        Ok(())
    }

    // Fix up branch offsets
    fn fixup_branches(&mut self) -> CompilerResult<()> {
        for (jmp_idx, target_rv_idx) in self.branch_fixups.clone() {
            let target_svm_idx = self.pc_map.get(&target_rv_idx)
                .ok_or_else(|| CompilerError::InvalidBranchTarget(target_rv_idx as i32))?;

            // Calculate offset: target - (current + 1)
            let offset = (*target_svm_idx as i64) - (jmp_idx as i64) - 1;
            if offset < i16::MIN as i64 || offset > i16::MAX as i64 {
                return Err(CompilerError::InvalidBranchTarget(target_rv_idx as i32));
            }

            self.output.instructions[jmp_idx].offset = offset as i16;
        }
        Ok(())
    }

    /// Emit return dispatch table for JALR instructions
    ///
    /// This creates a series of comparisons against known return sites
    /// and jumps to the appropriate SVM instruction.
    fn emit_return_dispatch(&mut self) -> CompilerResult<()> {
        if self.jalr_fixups.is_empty() {
            return Ok(());
        }

        // Record start of dispatch table
        let dispatch_start = self.output.len();

        // R1 contains the target byte address from JALR
        // We need to compare against each return site

        // First check for 0 (exit)
        self.output.push(SvmInstruction::jmp_imm(opcodes::JNE, SvmRegister::R1, 0, 1));
        self.output.push(SvmInstruction::exit());

        // For each return site, emit comparison and jump
        for &return_rv_idx in &self.return_sites.clone() {
            let return_byte_addr = (return_rv_idx * 4) as i32;
            let target_svm_idx = self.pc_map.get(&return_rv_idx)
                .ok_or_else(|| CompilerError::InvalidBranchTarget(return_rv_idx as i32))?;

            // Compare R1 with return address
            self.output.push(SvmInstruction::jmp_imm(opcodes::JNE, SvmRegister::R1, return_byte_addr, 1));

            // Emit jump to target SVM instruction
            let jmp_idx = self.output.len();
            self.output.push(SvmInstruction::ja(0));

            // Calculate offset for the jump
            let offset = (*target_svm_idx as i64) - (jmp_idx as i64) - 1;
            if offset < i16::MIN as i64 || offset > i16::MAX as i64 {
                return Err(CompilerError::InvalidBranchTarget(return_rv_idx as i32));
            }
            self.output.instructions[jmp_idx].offset = offset as i16;
        }

        // If no match found, exit (shouldn't happen in well-formed programs)
        self.output.push(SvmInstruction::exit());

        // Fix up all JALR jumps to point to dispatch table
        for jmp_idx in &self.jalr_fixups.clone() {
            let offset = (dispatch_start as i64) - (*jmp_idx as i64) - 1;
            if offset < i16::MIN as i64 || offset > i16::MAX as i64 {
                return Err(CompilerError::InvalidBranchTarget(dispatch_start as i32));
            }
            self.output.instructions[*jmp_idx].offset = offset as i16;
        }

        Ok(())
    }
}

impl Default for Compiler {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::svm::SvmVm;

    fn run_rv_program(instructions: &[Instruction]) -> u64 {
        let mut compiler = Compiler::new();
        let program = compiler.compile(instructions).unwrap();
        let mut vm = SvmVm::new(0x20000);
        vm.set_compute_limit(1_000_000);
        vm.execute(&program).unwrap()
    }

    #[test]
    fn test_simple_return() {
        // li a0, 42 (addi a0, x0, 42)
        // ecall (exit)
        let instructions = vec![
            Instruction::Addi { rd: Register::A0, rs1: Register::ZERO, imm: 42 },
            Instruction::Ecall,
        ];
        let result = run_rv_program(&instructions);
        assert_eq!(result, 42);
    }

    #[test]
    fn test_add() {
        // li a0, 10
        // li a1, 20
        // add a0, a0, a1
        // ecall
        let instructions = vec![
            Instruction::Addi { rd: Register::A0, rs1: Register::ZERO, imm: 10 },
            Instruction::Addi { rd: Register::A1, rs1: Register::ZERO, imm: 20 },
            Instruction::Add { rd: Register::A0, rs1: Register::A0, rs2: Register::A1 },
            Instruction::Ecall,
        ];
        let result = run_rv_program(&instructions);
        assert_eq!(result, 30);
    }

    #[test]
    fn test_mul() {
        // li a0, 6
        // li a1, 7
        // mul a0, a0, a1
        // ecall
        let instructions = vec![
            Instruction::Addi { rd: Register::A0, rs1: Register::ZERO, imm: 6 },
            Instruction::Addi { rd: Register::A1, rs1: Register::ZERO, imm: 7 },
            Instruction::Mul { rd: Register::A0, rs1: Register::A0, rs2: Register::A1 },
            Instruction::Ecall,
        ];
        let result = run_rv_program(&instructions);
        assert_eq!(result, 42);
    }

    #[test]
    fn test_branch() {
        // li a0, 5
        // li a1, 5
        // beq a0, a1, skip
        // li a0, 0
        // skip:
        // ecall
        let instructions = vec![
            Instruction::Addi { rd: Register::A0, rs1: Register::ZERO, imm: 5 },
            Instruction::Addi { rd: Register::A1, rs1: Register::ZERO, imm: 5 },
            Instruction::Beq { rs1: Register::A0, rs2: Register::A1, imm: 8 }, // Skip 2 instructions (8 bytes)
            Instruction::Addi { rd: Register::A0, rs1: Register::ZERO, imm: 0 },
            Instruction::Ecall,
        ];
        let result = run_rv_program(&instructions);
        assert_eq!(result, 5);
    }
}
