//! rv32im-svm: RISC-V rv32im to Solana SVM compiler
//!
//! This crate provides a compiler that translates RISC-V rv32im instructions
//! to Solana VM (sBPF) bytecode.
//!
//! # Example
//!
//! ```
//! use rv32im_svm::{compiler::Compiler, riscv::Instruction, riscv::Register, svm::SvmVm};
//!
//! // Create a simple program: return 42
//! let instructions = vec![
//!     Instruction::Addi { rd: Register::A0, rs1: Register::ZERO, imm: 42 },
//!     Instruction::Ecall,
//! ];
//!
//! // Compile to SVM
//! let mut compiler = Compiler::new();
//! let program = compiler.compile(&instructions).unwrap();
//!
//! // Execute
//! let mut vm = SvmVm::new(0x20000);
//! let result = vm.execute(&program).unwrap();
//! assert_eq!(result, 42);
//! ```

pub mod compiler;
pub mod riscv;
pub mod svm;

// Re-exports for convenience
pub use compiler::{Compiler, CompilerConfig, CompilerError, CompilerResult};
pub use riscv::{decode, Instruction, Register};
pub use svm::{SvmInstruction, SvmProgram, SvmRegister, SvmVm};

/// Compile RISC-V binary to SVM program
pub fn compile_binary(binary: &[u32]) -> CompilerResult<SvmProgram> {
    let mut compiler = Compiler::new();
    compiler.compile_binary(binary)
}

/// Compile RISC-V instructions to SVM program
pub fn compile(instructions: &[Instruction]) -> CompilerResult<SvmProgram> {
    let mut compiler = Compiler::new();
    compiler.compile(instructions)
}

/// Execute a compiled SVM program and return the result
pub fn execute(program: &SvmProgram) -> Result<u64, svm::SvmError> {
    let mut vm = SvmVm::new(0x20000); // 128KB memory
    vm.set_compute_limit(1_000_000);
    vm.execute(program)
}

/// Compile and execute RISC-V instructions
pub fn compile_and_execute(instructions: &[Instruction]) -> Result<u64, Box<dyn std::error::Error>> {
    let program = compile(instructions)?;
    let result = execute(&program)?;
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compile_and_execute() {
        let instructions = vec![
            Instruction::Addi { rd: Register::A0, rs1: Register::ZERO, imm: 100 },
            Instruction::Ecall,
        ];
        let result = compile_and_execute(&instructions).unwrap();
        assert_eq!(result, 100);
    }

    #[test]
    fn test_binary_compilation() {
        // addi a0, zero, 42 = 0x02a00513
        // ecall = 0x00000073
        let binary = vec![0x02a00513, 0x00000073];
        let program = compile_binary(&binary).unwrap();
        let result = execute(&program).unwrap();
        assert_eq!(result, 42);
    }
}
