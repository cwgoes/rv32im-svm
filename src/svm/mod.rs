//! SVM (Solana BPF) bytecode module

pub mod bytecode;
pub mod interpreter;

pub use bytecode::{opcodes, MemorySize, SvmInstruction, SvmProgram, SvmRegister};
pub use interpreter::{SvmError, SvmResult, SvmVm};
