//! SVM (Solana BPF) bytecode definitions
//!
//! Solana uses a modified version of eBPF called sBPF.
//! Key characteristics:
//! - 64-bit registers (r0-r10)
//! - r0: return value
//! - r1-r5: function arguments
//! - r6-r9: callee-saved
//! - r10: stack frame pointer (read-only)
//! - r11: internal use only
//! - 8-byte instructions (little-endian)

use std::fmt;

/// sBPF opcode classes
pub mod opcodes {
    // ALU operations (32-bit)
    pub const ALU32_IMM: u8 = 0x04;
    pub const ALU32_REG: u8 = 0x0c;

    // ALU operations (64-bit)
    pub const ALU64_IMM: u8 = 0x07;
    pub const ALU64_REG: u8 = 0x0f;

    // Jump operations
    pub const JMP_IMM: u8 = 0x05;
    pub const JMP_REG: u8 = 0x0d;
    pub const JMP32_IMM: u8 = 0x06;
    pub const JMP32_REG: u8 = 0x0e;

    // Memory operations
    pub const LD_IMM_DW: u8 = 0x18; // Load 64-bit immediate (lddw)
    pub const LD_ABS_B: u8 = 0x30;
    pub const LD_ABS_H: u8 = 0x28;
    pub const LD_ABS_W: u8 = 0x20;
    pub const LD_ABS_DW: u8 = 0x38;
    pub const LD_IND_B: u8 = 0x50;
    pub const LD_IND_H: u8 = 0x48;
    pub const LD_IND_W: u8 = 0x40;
    pub const LD_IND_DW: u8 = 0x58;
    pub const LDX_B: u8 = 0x71;
    pub const LDX_H: u8 = 0x69;
    pub const LDX_W: u8 = 0x61;
    pub const LDX_DW: u8 = 0x79;
    pub const ST_B: u8 = 0x72;
    pub const ST_H: u8 = 0x6a;
    pub const ST_W: u8 = 0x62;
    pub const ST_DW: u8 = 0x7a;
    pub const STX_B: u8 = 0x73;
    pub const STX_H: u8 = 0x6b;
    pub const STX_W: u8 = 0x63;
    pub const STX_DW: u8 = 0x7b;

    // ALU operation codes (combined with class)
    pub const ADD: u8 = 0x00;
    pub const SUB: u8 = 0x10;
    pub const MUL: u8 = 0x20;
    pub const DIV: u8 = 0x30;
    pub const OR: u8 = 0x40;
    pub const AND: u8 = 0x50;
    pub const LSH: u8 = 0x60;
    pub const RSH: u8 = 0x70;
    pub const NEG: u8 = 0x80;
    pub const MOD: u8 = 0x90;
    pub const XOR: u8 = 0xa0;
    pub const MOV: u8 = 0xb0;
    pub const ARSH: u8 = 0xc0; // Arithmetic right shift
    pub const END: u8 = 0xd0; // Endianness conversion

    // sBPF specific ALU operations
    pub const SDIV: u8 = 0xe0; // Signed division
    pub const SMOD: u8 = 0xf0; // Signed modulo (unofficial, using unused slot)

    // Jump operation codes
    pub const JA: u8 = 0x00;  // Jump always
    pub const JEQ: u8 = 0x10; // Jump if equal
    pub const JGT: u8 = 0x20; // Jump if greater than (unsigned)
    pub const JGE: u8 = 0x30; // Jump if greater or equal (unsigned)
    pub const JSET: u8 = 0x40; // Jump if (src & dst) != 0
    pub const JNE: u8 = 0x50; // Jump if not equal
    pub const JSGT: u8 = 0x60; // Jump if greater than (signed)
    pub const JSGE: u8 = 0x70; // Jump if greater or equal (signed)
    pub const CALL: u8 = 0x80; // Call function
    pub const EXIT: u8 = 0x90; // Exit program
    pub const JLT: u8 = 0xa0; // Jump if less than (unsigned)
    pub const JLE: u8 = 0xb0; // Jump if less or equal (unsigned)
    pub const JSLT: u8 = 0xc0; // Jump if less than (signed)
    pub const JSLE: u8 = 0xd0; // Jump if less or equal (signed)

    // Endianness operations for END
    pub const TO_LE: u8 = 0x00;
    pub const TO_BE: u8 = 0x08;
}

/// sBPF register
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SvmRegister(pub u8);

impl SvmRegister {
    pub const R0: SvmRegister = SvmRegister(0);  // Return value
    pub const R1: SvmRegister = SvmRegister(1);  // Arg 1
    pub const R2: SvmRegister = SvmRegister(2);  // Arg 2
    pub const R3: SvmRegister = SvmRegister(3);  // Arg 3
    pub const R4: SvmRegister = SvmRegister(4);  // Arg 4
    pub const R5: SvmRegister = SvmRegister(5);  // Arg 5
    pub const R6: SvmRegister = SvmRegister(6);  // Callee-saved
    pub const R7: SvmRegister = SvmRegister(7);  // Callee-saved
    pub const R8: SvmRegister = SvmRegister(8);  // Callee-saved
    pub const R9: SvmRegister = SvmRegister(9);  // Callee-saved
    pub const R10: SvmRegister = SvmRegister(10); // Stack frame pointer (read-only)

    pub fn new(val: u8) -> Self {
        assert!(val <= 10, "Register index out of range: {}", val);
        SvmRegister(val)
    }
}

impl fmt::Display for SvmRegister {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "r{}", self.0)
    }
}

/// sBPF instruction (8 bytes)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SvmInstruction {
    pub opcode: u8,
    pub dst: u8,
    pub src: u8,
    pub offset: i16,
    pub imm: i32,
}

impl SvmInstruction {
    pub fn new(opcode: u8, dst: u8, src: u8, offset: i16, imm: i32) -> Self {
        SvmInstruction { opcode, dst, src, offset, imm }
    }

    /// Encode instruction to bytes (little-endian)
    pub fn encode(&self) -> [u8; 8] {
        let mut bytes = [0u8; 8];
        bytes[0] = self.opcode;
        bytes[1] = (self.src << 4) | (self.dst & 0x0f);
        bytes[2..4].copy_from_slice(&self.offset.to_le_bytes());
        bytes[4..8].copy_from_slice(&self.imm.to_le_bytes());
        bytes
    }

    /// Decode instruction from bytes
    pub fn decode(bytes: &[u8; 8]) -> Self {
        let opcode = bytes[0];
        let dst = bytes[1] & 0x0f;
        let src = (bytes[1] >> 4) & 0x0f;
        let offset = i16::from_le_bytes([bytes[2], bytes[3]]);
        let imm = i32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]);
        SvmInstruction { opcode, dst, src, offset, imm }
    }

    // Constructor helpers

    /// ALU64 register operation: dst = dst OP src
    pub fn alu64_reg(op: u8, dst: SvmRegister, src: SvmRegister) -> Self {
        SvmInstruction::new(opcodes::ALU64_REG | op, dst.0, src.0, 0, 0)
    }

    /// ALU64 immediate operation: dst = dst OP imm
    pub fn alu64_imm(op: u8, dst: SvmRegister, imm: i32) -> Self {
        SvmInstruction::new(opcodes::ALU64_IMM | op, dst.0, 0, 0, imm)
    }

    /// ALU32 register operation: dst = (u32)dst OP (u32)src
    pub fn alu32_reg(op: u8, dst: SvmRegister, src: SvmRegister) -> Self {
        SvmInstruction::new(opcodes::ALU32_REG | op, dst.0, src.0, 0, 0)
    }

    /// ALU32 immediate operation: dst = (u32)dst OP (u32)imm
    pub fn alu32_imm(op: u8, dst: SvmRegister, imm: i32) -> Self {
        SvmInstruction::new(opcodes::ALU32_IMM | op, dst.0, 0, 0, imm)
    }

    /// Jump (unconditional)
    pub fn ja(offset: i16) -> Self {
        SvmInstruction::new(opcodes::JMP_IMM | opcodes::JA, 0, 0, offset, 0)
    }

    /// Jump conditional (register comparison)
    pub fn jmp_reg(op: u8, dst: SvmRegister, src: SvmRegister, offset: i16) -> Self {
        SvmInstruction::new(opcodes::JMP_REG | op, dst.0, src.0, offset, 0)
    }

    /// Jump conditional (immediate comparison)
    pub fn jmp_imm(op: u8, dst: SvmRegister, imm: i32, offset: i16) -> Self {
        SvmInstruction::new(opcodes::JMP_IMM | op, dst.0, 0, offset, imm)
    }

    /// Jump conditional 32-bit (register comparison)
    pub fn jmp32_reg(op: u8, dst: SvmRegister, src: SvmRegister, offset: i16) -> Self {
        SvmInstruction::new(opcodes::JMP32_REG | op, dst.0, src.0, offset, 0)
    }

    /// Jump conditional 32-bit (immediate comparison)
    pub fn jmp32_imm(op: u8, dst: SvmRegister, imm: i32, offset: i16) -> Self {
        SvmInstruction::new(opcodes::JMP32_IMM | op, dst.0, 0, offset, imm)
    }

    /// Call function
    pub fn call(func_id: i32) -> Self {
        SvmInstruction::new(opcodes::JMP_IMM | opcodes::CALL, 0, 0, 0, func_id)
    }

    /// Exit program
    pub fn exit() -> Self {
        SvmInstruction::new(opcodes::JMP_IMM | opcodes::EXIT, 0, 0, 0, 0)
    }

    /// Load 64-bit immediate (takes 2 instruction slots)
    pub fn lddw(dst: SvmRegister, imm: i64) -> [Self; 2] {
        [
            SvmInstruction::new(opcodes::LD_IMM_DW, dst.0, 0, 0, imm as i32),
            SvmInstruction::new(0, 0, 0, 0, (imm >> 32) as i32),
        ]
    }

    /// Load from memory: dst = *(size*)(src + offset)
    pub fn ldx(size: MemorySize, dst: SvmRegister, src: SvmRegister, offset: i16) -> Self {
        let opcode = match size {
            MemorySize::Byte => opcodes::LDX_B,
            MemorySize::Half => opcodes::LDX_H,
            MemorySize::Word => opcodes::LDX_W,
            MemorySize::DWord => opcodes::LDX_DW,
        };
        SvmInstruction::new(opcode, dst.0, src.0, offset, 0)
    }

    /// Store immediate to memory: *(size*)(dst + offset) = imm
    pub fn st(size: MemorySize, dst: SvmRegister, offset: i16, imm: i32) -> Self {
        let opcode = match size {
            MemorySize::Byte => opcodes::ST_B,
            MemorySize::Half => opcodes::ST_H,
            MemorySize::Word => opcodes::ST_W,
            MemorySize::DWord => opcodes::ST_DW,
        };
        SvmInstruction::new(opcode, dst.0, 0, offset, imm)
    }

    /// Store register to memory: *(size*)(dst + offset) = src
    pub fn stx(size: MemorySize, dst: SvmRegister, src: SvmRegister, offset: i16) -> Self {
        let opcode = match size {
            MemorySize::Byte => opcodes::STX_B,
            MemorySize::Half => opcodes::STX_H,
            MemorySize::Word => opcodes::STX_W,
            MemorySize::DWord => opcodes::STX_DW,
        };
        SvmInstruction::new(opcode, dst.0, src.0, offset, 0)
    }

    /// Move register: dst = src
    pub fn mov64_reg(dst: SvmRegister, src: SvmRegister) -> Self {
        Self::alu64_reg(opcodes::MOV, dst, src)
    }

    /// Move immediate: dst = imm
    pub fn mov64_imm(dst: SvmRegister, imm: i32) -> Self {
        Self::alu64_imm(opcodes::MOV, dst, imm)
    }

    /// Move 32-bit register: dst = (u32)src
    pub fn mov32_reg(dst: SvmRegister, src: SvmRegister) -> Self {
        Self::alu32_reg(opcodes::MOV, dst, src)
    }

    /// Move 32-bit immediate: dst = (u32)imm
    pub fn mov32_imm(dst: SvmRegister, imm: i32) -> Self {
        Self::alu32_imm(opcodes::MOV, dst, imm)
    }

    /// Add: dst += src
    pub fn add64_reg(dst: SvmRegister, src: SvmRegister) -> Self {
        Self::alu64_reg(opcodes::ADD, dst, src)
    }

    /// Add immediate: dst += imm
    pub fn add64_imm(dst: SvmRegister, imm: i32) -> Self {
        Self::alu64_imm(opcodes::ADD, dst, imm)
    }

    /// Sub: dst -= src
    pub fn sub64_reg(dst: SvmRegister, src: SvmRegister) -> Self {
        Self::alu64_reg(opcodes::SUB, dst, src)
    }

    /// Sub immediate: dst -= imm
    pub fn sub64_imm(dst: SvmRegister, imm: i32) -> Self {
        Self::alu64_imm(opcodes::SUB, dst, imm)
    }

    /// Mul: dst *= src
    pub fn mul64_reg(dst: SvmRegister, src: SvmRegister) -> Self {
        Self::alu64_reg(opcodes::MUL, dst, src)
    }

    /// Mul immediate: dst *= imm
    pub fn mul64_imm(dst: SvmRegister, imm: i32) -> Self {
        Self::alu64_imm(opcodes::MUL, dst, imm)
    }

    /// Div: dst /= src (unsigned)
    pub fn div64_reg(dst: SvmRegister, src: SvmRegister) -> Self {
        Self::alu64_reg(opcodes::DIV, dst, src)
    }

    /// Div immediate: dst /= imm (unsigned)
    pub fn div64_imm(dst: SvmRegister, imm: i32) -> Self {
        Self::alu64_imm(opcodes::DIV, dst, imm)
    }

    /// Signed div: dst /= src (signed)
    pub fn sdiv64_reg(dst: SvmRegister, src: SvmRegister) -> Self {
        Self::alu64_reg(opcodes::SDIV, dst, src)
    }

    /// Mod: dst %= src (unsigned)
    pub fn mod64_reg(dst: SvmRegister, src: SvmRegister) -> Self {
        Self::alu64_reg(opcodes::MOD, dst, src)
    }

    /// And: dst &= src
    pub fn and64_reg(dst: SvmRegister, src: SvmRegister) -> Self {
        Self::alu64_reg(opcodes::AND, dst, src)
    }

    /// And immediate: dst &= imm
    pub fn and64_imm(dst: SvmRegister, imm: i32) -> Self {
        Self::alu64_imm(opcodes::AND, dst, imm)
    }

    /// Or: dst |= src
    pub fn or64_reg(dst: SvmRegister, src: SvmRegister) -> Self {
        Self::alu64_reg(opcodes::OR, dst, src)
    }

    /// Or immediate: dst |= imm
    pub fn or64_imm(dst: SvmRegister, imm: i32) -> Self {
        Self::alu64_imm(opcodes::OR, dst, imm)
    }

    /// Xor: dst ^= src
    pub fn xor64_reg(dst: SvmRegister, src: SvmRegister) -> Self {
        Self::alu64_reg(opcodes::XOR, dst, src)
    }

    /// Xor immediate: dst ^= imm
    pub fn xor64_imm(dst: SvmRegister, imm: i32) -> Self {
        Self::alu64_imm(opcodes::XOR, dst, imm)
    }

    /// Left shift: dst <<= src
    pub fn lsh64_reg(dst: SvmRegister, src: SvmRegister) -> Self {
        Self::alu64_reg(opcodes::LSH, dst, src)
    }

    /// Left shift immediate: dst <<= imm
    pub fn lsh64_imm(dst: SvmRegister, imm: i32) -> Self {
        Self::alu64_imm(opcodes::LSH, dst, imm)
    }

    /// Right shift (logical): dst >>= src
    pub fn rsh64_reg(dst: SvmRegister, src: SvmRegister) -> Self {
        Self::alu64_reg(opcodes::RSH, dst, src)
    }

    /// Right shift immediate (logical): dst >>= imm
    pub fn rsh64_imm(dst: SvmRegister, imm: i32) -> Self {
        Self::alu64_imm(opcodes::RSH, dst, imm)
    }

    /// Arithmetic right shift: dst >>= src (signed)
    pub fn arsh64_reg(dst: SvmRegister, src: SvmRegister) -> Self {
        Self::alu64_reg(opcodes::ARSH, dst, src)
    }

    /// Arithmetic right shift immediate: dst >>= imm (signed)
    pub fn arsh64_imm(dst: SvmRegister, imm: i32) -> Self {
        Self::alu64_imm(opcodes::ARSH, dst, imm)
    }

    /// Negate: dst = -dst
    pub fn neg64(dst: SvmRegister) -> Self {
        Self::alu64_imm(opcodes::NEG, dst, 0)
    }

    // 32-bit ALU variants
    pub fn add32_reg(dst: SvmRegister, src: SvmRegister) -> Self {
        Self::alu32_reg(opcodes::ADD, dst, src)
    }

    pub fn add32_imm(dst: SvmRegister, imm: i32) -> Self {
        Self::alu32_imm(opcodes::ADD, dst, imm)
    }

    pub fn sub32_reg(dst: SvmRegister, src: SvmRegister) -> Self {
        Self::alu32_reg(opcodes::SUB, dst, src)
    }

    pub fn sub32_imm(dst: SvmRegister, imm: i32) -> Self {
        Self::alu32_imm(opcodes::SUB, dst, imm)
    }

    pub fn mul32_reg(dst: SvmRegister, src: SvmRegister) -> Self {
        Self::alu32_reg(opcodes::MUL, dst, src)
    }

    pub fn mul32_imm(dst: SvmRegister, imm: i32) -> Self {
        Self::alu32_imm(opcodes::MUL, dst, imm)
    }

    pub fn div32_reg(dst: SvmRegister, src: SvmRegister) -> Self {
        Self::alu32_reg(opcodes::DIV, dst, src)
    }

    pub fn sdiv32_reg(dst: SvmRegister, src: SvmRegister) -> Self {
        Self::alu32_reg(opcodes::SDIV, dst, src)
    }

    pub fn mod32_reg(dst: SvmRegister, src: SvmRegister) -> Self {
        Self::alu32_reg(opcodes::MOD, dst, src)
    }

    pub fn and32_reg(dst: SvmRegister, src: SvmRegister) -> Self {
        Self::alu32_reg(opcodes::AND, dst, src)
    }

    pub fn and32_imm(dst: SvmRegister, imm: i32) -> Self {
        Self::alu32_imm(opcodes::AND, dst, imm)
    }

    pub fn or32_reg(dst: SvmRegister, src: SvmRegister) -> Self {
        Self::alu32_reg(opcodes::OR, dst, src)
    }

    pub fn or32_imm(dst: SvmRegister, imm: i32) -> Self {
        Self::alu32_imm(opcodes::OR, dst, imm)
    }

    pub fn xor32_reg(dst: SvmRegister, src: SvmRegister) -> Self {
        Self::alu32_reg(opcodes::XOR, dst, src)
    }

    pub fn xor32_imm(dst: SvmRegister, imm: i32) -> Self {
        Self::alu32_imm(opcodes::XOR, dst, imm)
    }

    pub fn lsh32_reg(dst: SvmRegister, src: SvmRegister) -> Self {
        Self::alu32_reg(opcodes::LSH, dst, src)
    }

    pub fn lsh32_imm(dst: SvmRegister, imm: i32) -> Self {
        Self::alu32_imm(opcodes::LSH, dst, imm)
    }

    pub fn rsh32_reg(dst: SvmRegister, src: SvmRegister) -> Self {
        Self::alu32_reg(opcodes::RSH, dst, src)
    }

    pub fn rsh32_imm(dst: SvmRegister, imm: i32) -> Self {
        Self::alu32_imm(opcodes::RSH, dst, imm)
    }

    pub fn arsh32_reg(dst: SvmRegister, src: SvmRegister) -> Self {
        Self::alu32_reg(opcodes::ARSH, dst, src)
    }

    pub fn arsh32_imm(dst: SvmRegister, imm: i32) -> Self {
        Self::alu32_imm(opcodes::ARSH, dst, imm)
    }
}

impl fmt::Display for SvmInstruction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let class = self.opcode & 0x07;
        let op = self.opcode & 0xf0;

        match class {
            0x04 | 0x07 => {
                // ALU32 or ALU64 immediate
                let width = if class == 0x04 { "32" } else { "64" };
                let op_name = alu_op_name(op);
                write!(f, "{}{} r{}, {}", op_name, width, self.dst, self.imm)
            }
            0x0c | 0x0f => {
                // ALU32 or ALU64 register
                let width = if class == 0x0c { "32" } else { "64" };
                let op_name = alu_op_name(op);
                write!(f, "{}{} r{}, r{}", op_name, width, self.dst, self.src)
            }
            0x05 => {
                // JMP immediate
                match op {
                    opcodes::JA => write!(f, "ja {:+}", self.offset),
                    opcodes::CALL => write!(f, "call {}", self.imm),
                    opcodes::EXIT => write!(f, "exit"),
                    _ => {
                        let op_name = jmp_op_name(op);
                        write!(f, "{} r{}, {}, {:+}", op_name, self.dst, self.imm, self.offset)
                    }
                }
            }
            0x0d => {
                // JMP register
                let op_name = jmp_op_name(op);
                write!(f, "{} r{}, r{}, {:+}", op_name, self.dst, self.src, self.offset)
            }
            _ => {
                // Memory or other
                write!(f, "op={:#04x} dst={} src={} off={} imm={}",
                    self.opcode, self.dst, self.src, self.offset, self.imm)
            }
        }
    }
}

fn alu_op_name(op: u8) -> &'static str {
    match op {
        opcodes::ADD => "add",
        opcodes::SUB => "sub",
        opcodes::MUL => "mul",
        opcodes::DIV => "div",
        opcodes::OR => "or",
        opcodes::AND => "and",
        opcodes::LSH => "lsh",
        opcodes::RSH => "rsh",
        opcodes::NEG => "neg",
        opcodes::MOD => "mod",
        opcodes::XOR => "xor",
        opcodes::MOV => "mov",
        opcodes::ARSH => "arsh",
        opcodes::SDIV => "sdiv",
        _ => "unknown",
    }
}

fn jmp_op_name(op: u8) -> &'static str {
    match op {
        opcodes::JEQ => "jeq",
        opcodes::JGT => "jgt",
        opcodes::JGE => "jge",
        opcodes::JSET => "jset",
        opcodes::JNE => "jne",
        opcodes::JSGT => "jsgt",
        opcodes::JSGE => "jsge",
        opcodes::JLT => "jlt",
        opcodes::JLE => "jle",
        opcodes::JSLT => "jslt",
        opcodes::JSLE => "jsle",
        _ => "unknown",
    }
}

/// Memory access size
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemorySize {
    Byte,   // 8-bit
    Half,   // 16-bit
    Word,   // 32-bit
    DWord,  // 64-bit
}

/// SVM program
#[derive(Debug, Clone)]
pub struct SvmProgram {
    pub instructions: Vec<SvmInstruction>,
}

impl SvmProgram {
    pub fn new() -> Self {
        SvmProgram {
            instructions: Vec::new(),
        }
    }

    pub fn push(&mut self, inst: SvmInstruction) {
        self.instructions.push(inst);
    }

    pub fn extend(&mut self, insts: impl IntoIterator<Item = SvmInstruction>) {
        self.instructions.extend(insts);
    }

    /// Encode program to bytes
    pub fn encode(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(self.instructions.len() * 8);
        for inst in &self.instructions {
            bytes.extend_from_slice(&inst.encode());
        }
        bytes
    }

    /// Get number of instructions
    pub fn len(&self) -> usize {
        self.instructions.len()
    }

    /// Check if program is empty
    pub fn is_empty(&self) -> bool {
        self.instructions.is_empty()
    }

    /// Calculate compute cost (simplified model)
    /// In real Solana, compute units are more complex
    pub fn compute_cost(&self) -> u64 {
        let mut cost = 0u64;
        for inst in &self.instructions {
            cost += instruction_cost(inst);
        }
        cost
    }
}

impl Default for SvmProgram {
    fn default() -> Self {
        Self::new()
    }
}

/// Compute unit cost for each instruction type
/// Based on Solana's compute budget
fn instruction_cost(inst: &SvmInstruction) -> u64 {
    let class = inst.opcode & 0x07;
    let op = inst.opcode & 0xf0;

    match class {
        // ALU operations
        0x04 | 0x07 | 0x0c | 0x0f => {
            match op {
                opcodes::MUL => 10,
                opcodes::DIV | opcodes::MOD | opcodes::SDIV => 25,
                _ => 1,
            }
        }
        // Jump operations
        0x05 | 0x0d => {
            match op {
                opcodes::CALL => 100,
                opcodes::EXIT => 1,
                _ => 1, // Branches
            }
        }
        // Memory operations
        _ => {
            if inst.opcode == opcodes::LD_IMM_DW {
                1
            } else {
                5 // Memory access
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_instruction_encode_decode() {
        let inst = SvmInstruction::add64_imm(SvmRegister::R1, 42);
        let bytes = inst.encode();
        let decoded = SvmInstruction::decode(&bytes);
        assert_eq!(inst, decoded);
    }

    #[test]
    fn test_mov_imm() {
        let inst = SvmInstruction::mov64_imm(SvmRegister::R0, 100);
        assert_eq!(inst.opcode, opcodes::ALU64_IMM | opcodes::MOV);
        assert_eq!(inst.dst, 0);
        assert_eq!(inst.imm, 100);
    }

    #[test]
    fn test_program_encode() {
        let mut prog = SvmProgram::new();
        prog.push(SvmInstruction::mov64_imm(SvmRegister::R0, 42));
        prog.push(SvmInstruction::exit());

        let bytes = prog.encode();
        assert_eq!(bytes.len(), 16);
    }
}
