//! SVM interpreter for testing compiled programs
//!
//! This is a simplified interpreter for sBPF/SVM bytecode.
//! It's used for testing the rv32im to SVM compiler.

use super::bytecode::{opcodes, MemorySize, SvmInstruction, SvmProgram};
use thiserror::Error;

/// SVM execution error
#[derive(Debug, Error)]
pub enum SvmError {
    #[error("Division by zero")]
    DivisionByZero,

    #[error("Invalid opcode: {0:#04x}")]
    InvalidOpcode(u8),

    #[error("Out of bounds memory access at address {0:#x}")]
    MemoryOutOfBounds(u64),

    #[error("Stack overflow")]
    StackOverflow,

    #[error("Stack underflow")]
    StackUnderflow,

    #[error("Invalid program counter: {0}")]
    InvalidPC(usize),

    #[error("Compute budget exceeded: used {0}, limit {1}")]
    ComputeBudgetExceeded(u64, u64),

    #[error("Syscall {0} not implemented")]
    SyscallNotImplemented(i32),

    #[error("Program halted with exit code {0}")]
    Halted(u64),

    #[error("Write to read-only register r10")]
    WriteToR10,
}

/// Result type for SVM operations
pub type SvmResult<T> = Result<T, SvmError>;

/// SVM virtual machine state
pub struct SvmVm {
    /// Registers r0-r10
    pub registers: [u64; 11],

    /// Memory (flat memory model for simplicity)
    pub memory: Vec<u8>,

    /// Program counter (instruction index)
    pub pc: usize,

    /// Compute units used
    pub compute_units: u64,

    /// Compute unit limit
    pub compute_limit: u64,

    /// Stack pointer (within memory)
    pub stack_base: u64,

    /// Whether execution has halted
    pub halted: bool,
}

impl SvmVm {
    /// Create a new VM with given memory size
    pub fn new(memory_size: usize) -> Self {
        let mut vm = SvmVm {
            registers: [0; 11],
            memory: vec![0; memory_size],
            pc: 0,
            compute_units: 0,
            compute_limit: 200_000, // Default compute limit
            stack_base: memory_size as u64,
            halted: false,
        };
        // r10 is the stack frame pointer
        vm.registers[10] = vm.stack_base;
        vm
    }

    /// Set compute unit limit
    pub fn set_compute_limit(&mut self, limit: u64) {
        self.compute_limit = limit;
    }

    /// Get register value
    pub fn get_reg(&self, reg: u8) -> u64 {
        self.registers[reg as usize]
    }

    /// Set register value (r10 is read-only)
    pub fn set_reg(&mut self, reg: u8, value: u64) -> SvmResult<()> {
        if reg == 10 {
            return Err(SvmError::WriteToR10);
        }
        self.registers[reg as usize] = value;
        Ok(())
    }

    /// Read memory
    pub fn read_mem(&self, addr: u64, size: MemorySize) -> SvmResult<u64> {
        let addr = addr as usize;
        let mem_size = match size {
            MemorySize::Byte => 1,
            MemorySize::Half => 2,
            MemorySize::Word => 4,
            MemorySize::DWord => 8,
        };

        if addr + mem_size > self.memory.len() {
            return Err(SvmError::MemoryOutOfBounds(addr as u64));
        }

        let value = match size {
            MemorySize::Byte => self.memory[addr] as u64,
            MemorySize::Half => {
                u16::from_le_bytes([self.memory[addr], self.memory[addr + 1]]) as u64
            }
            MemorySize::Word => {
                u32::from_le_bytes([
                    self.memory[addr],
                    self.memory[addr + 1],
                    self.memory[addr + 2],
                    self.memory[addr + 3],
                ]) as u64
            }
            MemorySize::DWord => {
                u64::from_le_bytes([
                    self.memory[addr],
                    self.memory[addr + 1],
                    self.memory[addr + 2],
                    self.memory[addr + 3],
                    self.memory[addr + 4],
                    self.memory[addr + 5],
                    self.memory[addr + 6],
                    self.memory[addr + 7],
                ])
            }
        };

        Ok(value)
    }

    /// Write memory
    pub fn write_mem(&mut self, addr: u64, size: MemorySize, value: u64) -> SvmResult<()> {
        let addr = addr as usize;
        let mem_size = match size {
            MemorySize::Byte => 1,
            MemorySize::Half => 2,
            MemorySize::Word => 4,
            MemorySize::DWord => 8,
        };

        if addr + mem_size > self.memory.len() {
            return Err(SvmError::MemoryOutOfBounds(addr as u64));
        }

        match size {
            MemorySize::Byte => {
                self.memory[addr] = value as u8;
            }
            MemorySize::Half => {
                let bytes = (value as u16).to_le_bytes();
                self.memory[addr..addr + 2].copy_from_slice(&bytes);
            }
            MemorySize::Word => {
                let bytes = (value as u32).to_le_bytes();
                self.memory[addr..addr + 4].copy_from_slice(&bytes);
            }
            MemorySize::DWord => {
                let bytes = value.to_le_bytes();
                self.memory[addr..addr + 8].copy_from_slice(&bytes);
            }
        }

        Ok(())
    }

    /// Execute a program
    pub fn execute(&mut self, program: &SvmProgram) -> SvmResult<u64> {
        self.pc = 0;
        self.halted = false;

        while !self.halted && self.pc < program.len() {
            let inst = &program.instructions[self.pc];
            self.execute_instruction(inst, program)?;
        }

        Ok(self.registers[0])
    }

    /// Execute a single instruction
    fn execute_instruction(&mut self, inst: &SvmInstruction, program: &SvmProgram) -> SvmResult<()> {
        // Check compute budget
        let cost = self.instruction_cost(inst);
        self.compute_units += cost;
        if self.compute_units > self.compute_limit {
            return Err(SvmError::ComputeBudgetExceeded(self.compute_units, self.compute_limit));
        }

        // In eBPF, opcode is structured as: [op (4 bits)][src (1 bit)][class (3 bits)]
        // But for matching, we need to consider bits 0-3 (class + src bit)
        let opcode_class = inst.opcode & 0x0f;
        let op = inst.opcode & 0xf0;

        match opcode_class {
            // ALU64 immediate (class=0x07, src=0) -> 0x07
            0x07 => {
                self.execute_alu64_imm(op, inst.dst, inst.imm)?;
                self.pc += 1;
            }

            // ALU64 register (class=0x07, src=1) -> 0x0f
            0x0f => {
                self.execute_alu64_reg(op, inst.dst, inst.src)?;
                self.pc += 1;
            }

            // ALU32 immediate (class=0x04, src=0) -> 0x04
            0x04 => {
                self.execute_alu32_imm(op, inst.dst, inst.imm)?;
                self.pc += 1;
            }

            // ALU32 register (class=0x04, src=1) -> 0x0c
            0x0c => {
                self.execute_alu32_reg(op, inst.dst, inst.src)?;
                self.pc += 1;
            }

            // JMP immediate (class=0x05, src=0) -> 0x05
            0x05 => {
                self.execute_jmp_imm(op, inst, program)?;
            }

            // JMP register (class=0x05, src=1) -> 0x0d
            0x0d => {
                self.execute_jmp_reg(op, inst)?;
            }

            // JMP32 immediate (class=0x06, src=0) -> 0x06
            0x06 => {
                self.execute_jmp32_imm(op, inst)?;
            }

            // JMP32 register (class=0x06, src=1) -> 0x0e
            0x0e => {
                self.execute_jmp32_reg(op, inst)?;
            }

            // Memory operations
            _ => {
                self.execute_memory(inst)?;
                self.pc += 1;
            }
        }

        Ok(())
    }

    fn execute_alu64_imm(&mut self, op: u8, dst: u8, imm: i32) -> SvmResult<()> {
        let dst_val = self.get_reg(dst);
        let imm_val = imm as i64 as u64; // Sign extend to 64-bit

        let result = match op {
            opcodes::ADD => dst_val.wrapping_add(imm_val),
            opcodes::SUB => dst_val.wrapping_sub(imm_val),
            opcodes::MUL => dst_val.wrapping_mul(imm_val),
            opcodes::DIV => {
                if imm_val == 0 {
                    return Err(SvmError::DivisionByZero);
                }
                dst_val / imm_val
            }
            opcodes::SDIV => {
                if imm_val == 0 {
                    return Err(SvmError::DivisionByZero);
                }
                ((dst_val as i64) / (imm_val as i64)) as u64
            }
            opcodes::MOD => {
                if imm_val == 0 {
                    return Err(SvmError::DivisionByZero);
                }
                dst_val % imm_val
            }
            opcodes::OR => dst_val | imm_val,
            opcodes::AND => dst_val & imm_val,
            opcodes::LSH => dst_val << (imm_val & 63),
            opcodes::RSH => dst_val >> (imm_val & 63),
            opcodes::ARSH => ((dst_val as i64) >> (imm_val & 63)) as u64,
            opcodes::XOR => dst_val ^ imm_val,
            opcodes::MOV => imm_val,
            opcodes::NEG => (-(dst_val as i64)) as u64,
            _ => return Err(SvmError::InvalidOpcode(op)),
        };

        self.set_reg(dst, result)
    }

    fn execute_alu64_reg(&mut self, op: u8, dst: u8, src: u8) -> SvmResult<()> {
        let dst_val = self.get_reg(dst);
        let src_val = self.get_reg(src);

        let result = match op {
            opcodes::ADD => dst_val.wrapping_add(src_val),
            opcodes::SUB => dst_val.wrapping_sub(src_val),
            opcodes::MUL => dst_val.wrapping_mul(src_val),
            opcodes::DIV => {
                if src_val == 0 {
                    return Err(SvmError::DivisionByZero);
                }
                dst_val / src_val
            }
            opcodes::SDIV => {
                if src_val == 0 {
                    return Err(SvmError::DivisionByZero);
                }
                ((dst_val as i64) / (src_val as i64)) as u64
            }
            opcodes::MOD => {
                if src_val == 0 {
                    return Err(SvmError::DivisionByZero);
                }
                dst_val % src_val
            }
            opcodes::OR => dst_val | src_val,
            opcodes::AND => dst_val & src_val,
            opcodes::LSH => dst_val << (src_val & 63),
            opcodes::RSH => dst_val >> (src_val & 63),
            opcodes::ARSH => ((dst_val as i64) >> (src_val & 63)) as u64,
            opcodes::XOR => dst_val ^ src_val,
            opcodes::MOV => src_val,
            _ => return Err(SvmError::InvalidOpcode(op)),
        };

        self.set_reg(dst, result)
    }

    fn execute_alu32_imm(&mut self, op: u8, dst: u8, imm: i32) -> SvmResult<()> {
        let dst_val = self.get_reg(dst) as u32;
        let imm_val = imm as u32;

        let result = match op {
            opcodes::ADD => dst_val.wrapping_add(imm_val),
            opcodes::SUB => dst_val.wrapping_sub(imm_val),
            opcodes::MUL => dst_val.wrapping_mul(imm_val),
            opcodes::DIV => {
                if imm_val == 0 {
                    return Err(SvmError::DivisionByZero);
                }
                dst_val / imm_val
            }
            opcodes::SDIV => {
                if imm_val == 0 {
                    return Err(SvmError::DivisionByZero);
                }
                ((dst_val as i32) / (imm_val as i32)) as u32
            }
            opcodes::MOD => {
                if imm_val == 0 {
                    return Err(SvmError::DivisionByZero);
                }
                dst_val % imm_val
            }
            opcodes::OR => dst_val | imm_val,
            opcodes::AND => dst_val & imm_val,
            opcodes::LSH => dst_val << (imm_val & 31),
            opcodes::RSH => dst_val >> (imm_val & 31),
            opcodes::ARSH => ((dst_val as i32) >> (imm_val & 31)) as u32,
            opcodes::XOR => dst_val ^ imm_val,
            opcodes::MOV => imm_val,
            opcodes::NEG => (-(dst_val as i32)) as u32,
            _ => return Err(SvmError::InvalidOpcode(op)),
        };

        // 32-bit operations zero-extend to 64-bit
        self.set_reg(dst, result as u64)
    }

    fn execute_alu32_reg(&mut self, op: u8, dst: u8, src: u8) -> SvmResult<()> {
        let dst_val = self.get_reg(dst) as u32;
        let src_val = self.get_reg(src) as u32;

        let result = match op {
            opcodes::ADD => dst_val.wrapping_add(src_val),
            opcodes::SUB => dst_val.wrapping_sub(src_val),
            opcodes::MUL => dst_val.wrapping_mul(src_val),
            opcodes::DIV => {
                if src_val == 0 {
                    return Err(SvmError::DivisionByZero);
                }
                dst_val / src_val
            }
            opcodes::SDIV => {
                if src_val == 0 {
                    return Err(SvmError::DivisionByZero);
                }
                ((dst_val as i32) / (src_val as i32)) as u32
            }
            opcodes::MOD => {
                if src_val == 0 {
                    return Err(SvmError::DivisionByZero);
                }
                dst_val % src_val
            }
            opcodes::OR => dst_val | src_val,
            opcodes::AND => dst_val & src_val,
            opcodes::LSH => dst_val << (src_val & 31),
            opcodes::RSH => dst_val >> (src_val & 31),
            opcodes::ARSH => ((dst_val as i32) >> (src_val & 31)) as u32,
            opcodes::XOR => dst_val ^ src_val,
            opcodes::MOV => src_val,
            _ => return Err(SvmError::InvalidOpcode(op)),
        };

        // 32-bit operations zero-extend to 64-bit
        self.set_reg(dst, result as u64)
    }

    fn execute_jmp_imm(&mut self, op: u8, inst: &SvmInstruction, _program: &SvmProgram) -> SvmResult<()> {
        match op {
            opcodes::JA => {
                self.pc = (self.pc as i64 + 1 + inst.offset as i64) as usize;
            }
            opcodes::EXIT => {
                self.halted = true;
            }
            opcodes::CALL => {
                // Syscall - not fully implemented
                return Err(SvmError::SyscallNotImplemented(inst.imm));
            }
            _ => {
                let dst_val = self.get_reg(inst.dst);
                let imm_val = inst.imm as i64 as u64;
                let cond = match op {
                    opcodes::JEQ => dst_val == imm_val,
                    opcodes::JNE => dst_val != imm_val,
                    opcodes::JGT => dst_val > imm_val,
                    opcodes::JGE => dst_val >= imm_val,
                    opcodes::JLT => dst_val < imm_val,
                    opcodes::JLE => dst_val <= imm_val,
                    opcodes::JSGT => (dst_val as i64) > (imm_val as i64),
                    opcodes::JSGE => (dst_val as i64) >= (imm_val as i64),
                    opcodes::JSLT => (dst_val as i64) < (imm_val as i64),
                    opcodes::JSLE => (dst_val as i64) <= (imm_val as i64),
                    opcodes::JSET => (dst_val & imm_val) != 0,
                    _ => return Err(SvmError::InvalidOpcode(op)),
                };

                if cond {
                    self.pc = (self.pc as i64 + 1 + inst.offset as i64) as usize;
                } else {
                    self.pc += 1;
                }
            }
        }
        Ok(())
    }

    fn execute_jmp_reg(&mut self, op: u8, inst: &SvmInstruction) -> SvmResult<()> {
        let dst_val = self.get_reg(inst.dst);
        let src_val = self.get_reg(inst.src);

        let cond = match op {
            opcodes::JEQ => dst_val == src_val,
            opcodes::JNE => dst_val != src_val,
            opcodes::JGT => dst_val > src_val,
            opcodes::JGE => dst_val >= src_val,
            opcodes::JLT => dst_val < src_val,
            opcodes::JLE => dst_val <= src_val,
            opcodes::JSGT => (dst_val as i64) > (src_val as i64),
            opcodes::JSGE => (dst_val as i64) >= (src_val as i64),
            opcodes::JSLT => (dst_val as i64) < (src_val as i64),
            opcodes::JSLE => (dst_val as i64) <= (src_val as i64),
            opcodes::JSET => (dst_val & src_val) != 0,
            _ => return Err(SvmError::InvalidOpcode(op)),
        };

        if cond {
            self.pc = (self.pc as i64 + 1 + inst.offset as i64) as usize;
        } else {
            self.pc += 1;
        }
        Ok(())
    }

    fn execute_jmp32_imm(&mut self, op: u8, inst: &SvmInstruction) -> SvmResult<()> {
        let dst_val = self.get_reg(inst.dst) as u32;
        let imm_val = inst.imm as u32;

        let cond = match op {
            opcodes::JEQ => dst_val == imm_val,
            opcodes::JNE => dst_val != imm_val,
            opcodes::JGT => dst_val > imm_val,
            opcodes::JGE => dst_val >= imm_val,
            opcodes::JLT => dst_val < imm_val,
            opcodes::JLE => dst_val <= imm_val,
            opcodes::JSGT => (dst_val as i32) > (imm_val as i32),
            opcodes::JSGE => (dst_val as i32) >= (imm_val as i32),
            opcodes::JSLT => (dst_val as i32) < (imm_val as i32),
            opcodes::JSLE => (dst_val as i32) <= (imm_val as i32),
            opcodes::JSET => (dst_val & imm_val) != 0,
            _ => return Err(SvmError::InvalidOpcode(op)),
        };

        if cond {
            self.pc = (self.pc as i64 + 1 + inst.offset as i64) as usize;
        } else {
            self.pc += 1;
        }
        Ok(())
    }

    fn execute_jmp32_reg(&mut self, op: u8, inst: &SvmInstruction) -> SvmResult<()> {
        let dst_val = self.get_reg(inst.dst) as u32;
        let src_val = self.get_reg(inst.src) as u32;

        let cond = match op {
            opcodes::JEQ => dst_val == src_val,
            opcodes::JNE => dst_val != src_val,
            opcodes::JGT => dst_val > src_val,
            opcodes::JGE => dst_val >= src_val,
            opcodes::JLT => dst_val < src_val,
            opcodes::JLE => dst_val <= src_val,
            opcodes::JSGT => (dst_val as i32) > (src_val as i32),
            opcodes::JSGE => (dst_val as i32) >= (src_val as i32),
            opcodes::JSLT => (dst_val as i32) < (src_val as i32),
            opcodes::JSLE => (dst_val as i32) <= (src_val as i32),
            opcodes::JSET => (dst_val & src_val) != 0,
            _ => return Err(SvmError::InvalidOpcode(op)),
        };

        if cond {
            self.pc = (self.pc as i64 + 1 + inst.offset as i64) as usize;
        } else {
            self.pc += 1;
        }
        Ok(())
    }

    fn execute_memory(&mut self, inst: &SvmInstruction) -> SvmResult<()> {
        match inst.opcode {
            opcodes::LD_IMM_DW => {
                // 64-bit immediate load (takes 2 instructions)
                // This is handled specially - the next instruction contains upper 32 bits
                // For now, just load the 32-bit immediate
                self.set_reg(inst.dst, inst.imm as i64 as u64)?;
            }
            opcodes::LDX_B => {
                let addr = self.get_reg(inst.src).wrapping_add(inst.offset as i64 as u64);
                let val = self.read_mem(addr, MemorySize::Byte)?;
                self.set_reg(inst.dst, val)?;
            }
            opcodes::LDX_H => {
                let addr = self.get_reg(inst.src).wrapping_add(inst.offset as i64 as u64);
                let val = self.read_mem(addr, MemorySize::Half)?;
                self.set_reg(inst.dst, val)?;
            }
            opcodes::LDX_W => {
                let addr = self.get_reg(inst.src).wrapping_add(inst.offset as i64 as u64);
                let val = self.read_mem(addr, MemorySize::Word)?;
                self.set_reg(inst.dst, val)?;
            }
            opcodes::LDX_DW => {
                let addr = self.get_reg(inst.src).wrapping_add(inst.offset as i64 as u64);
                let val = self.read_mem(addr, MemorySize::DWord)?;
                self.set_reg(inst.dst, val)?;
            }
            opcodes::ST_B => {
                let addr = self.get_reg(inst.dst).wrapping_add(inst.offset as i64 as u64);
                self.write_mem(addr, MemorySize::Byte, inst.imm as u64)?;
            }
            opcodes::ST_H => {
                let addr = self.get_reg(inst.dst).wrapping_add(inst.offset as i64 as u64);
                self.write_mem(addr, MemorySize::Half, inst.imm as u64)?;
            }
            opcodes::ST_W => {
                let addr = self.get_reg(inst.dst).wrapping_add(inst.offset as i64 as u64);
                self.write_mem(addr, MemorySize::Word, inst.imm as u64)?;
            }
            opcodes::ST_DW => {
                let addr = self.get_reg(inst.dst).wrapping_add(inst.offset as i64 as u64);
                self.write_mem(addr, MemorySize::DWord, inst.imm as u64)?;
            }
            opcodes::STX_B => {
                let addr = self.get_reg(inst.dst).wrapping_add(inst.offset as i64 as u64);
                let val = self.get_reg(inst.src);
                self.write_mem(addr, MemorySize::Byte, val)?;
            }
            opcodes::STX_H => {
                let addr = self.get_reg(inst.dst).wrapping_add(inst.offset as i64 as u64);
                let val = self.get_reg(inst.src);
                self.write_mem(addr, MemorySize::Half, val)?;
            }
            opcodes::STX_W => {
                let addr = self.get_reg(inst.dst).wrapping_add(inst.offset as i64 as u64);
                let val = self.get_reg(inst.src);
                self.write_mem(addr, MemorySize::Word, val)?;
            }
            opcodes::STX_DW => {
                let addr = self.get_reg(inst.dst).wrapping_add(inst.offset as i64 as u64);
                let val = self.get_reg(inst.src);
                self.write_mem(addr, MemorySize::DWord, val)?;
            }
            _ => return Err(SvmError::InvalidOpcode(inst.opcode)),
        }
        Ok(())
    }

    fn instruction_cost(&self, inst: &SvmInstruction) -> u64 {
        let opcode_class = inst.opcode & 0x0f;
        let op = inst.opcode & 0xf0;

        match opcode_class {
            // ALU operations
            0x04 | 0x07 | 0x0c | 0x0f => {
                match op {
                    opcodes::MUL => 10,
                    opcodes::DIV | opcodes::MOD | opcodes::SDIV => 25,
                    _ => 1,
                }
            }
            // Jump operations
            0x05 | 0x0d | 0x06 | 0x0e => {
                match op {
                    opcodes::CALL => 100,
                    opcodes::EXIT => 1,
                    _ => 1,
                }
            }
            _ => 5, // Memory operations
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::bytecode::SvmRegister;

    #[test]
    fn test_simple_program() {
        let mut program = SvmProgram::new();
        program.push(SvmInstruction::mov64_imm(SvmRegister::R0, 42));
        program.push(SvmInstruction::exit());

        let mut vm = SvmVm::new(4096);
        let result = vm.execute(&program).unwrap();
        assert_eq!(result, 42);
    }

    #[test]
    fn test_arithmetic() {
        let mut program = SvmProgram::new();
        program.push(SvmInstruction::mov64_imm(SvmRegister::R0, 10));
        program.push(SvmInstruction::add64_imm(SvmRegister::R0, 5));
        program.push(SvmInstruction::exit());

        let mut vm = SvmVm::new(4096);
        let result = vm.execute(&program).unwrap();
        assert_eq!(result, 15);
    }

    #[test]
    fn test_conditional_jump() {
        let mut program = SvmProgram::new();
        program.push(SvmInstruction::mov64_imm(SvmRegister::R0, 5));
        program.push(SvmInstruction::mov64_imm(SvmRegister::R1, 5));
        // jeq r0, r1, +1 (skip next instruction)
        program.push(SvmInstruction::jmp_reg(opcodes::JEQ, SvmRegister::R0, SvmRegister::R1, 1));
        program.push(SvmInstruction::mov64_imm(SvmRegister::R0, 0)); // Should be skipped
        program.push(SvmInstruction::exit());

        let mut vm = SvmVm::new(4096);
        let result = vm.execute(&program).unwrap();
        assert_eq!(result, 5);
    }

    #[test]
    fn test_memory_operations() {
        let mut program = SvmProgram::new();
        // Store 42 at address 100
        program.push(SvmInstruction::mov64_imm(SvmRegister::R1, 100));
        program.push(SvmInstruction::st(MemorySize::Word, SvmRegister::R1, 0, 42));
        // Load it back
        program.push(SvmInstruction::ldx(MemorySize::Word, SvmRegister::R0, SvmRegister::R1, 0));
        program.push(SvmInstruction::exit());

        let mut vm = SvmVm::new(4096);
        let result = vm.execute(&program).unwrap();
        assert_eq!(result, 42);
    }

    #[test]
    fn test_multiplication() {
        let mut program = SvmProgram::new();
        program.push(SvmInstruction::mov64_imm(SvmRegister::R0, 6));
        program.push(SvmInstruction::mul64_imm(SvmRegister::R0, 7));
        program.push(SvmInstruction::exit());

        let mut vm = SvmVm::new(4096);
        let result = vm.execute(&program).unwrap();
        assert_eq!(result, 42);
    }
}
