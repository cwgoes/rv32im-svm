//! RISC-V rv32im instruction set

pub mod decoder;
pub mod instruction;

pub use decoder::decode;
pub use instruction::{Instruction, Register};
