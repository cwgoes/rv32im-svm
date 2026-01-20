//! RISC-V rv32im instruction definitions

use std::fmt;

/// RISC-V register (x0-x31)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Register(pub u8);

impl Register {
    pub const ZERO: Register = Register(0);
    pub const RA: Register = Register(1);
    pub const SP: Register = Register(2);
    pub const GP: Register = Register(3);
    pub const TP: Register = Register(4);
    pub const T0: Register = Register(5);
    pub const T1: Register = Register(6);
    pub const T2: Register = Register(7);
    pub const S0: Register = Register(8);
    pub const FP: Register = Register(8);
    pub const S1: Register = Register(9);
    pub const A0: Register = Register(10);
    pub const A1: Register = Register(11);
    pub const A2: Register = Register(12);
    pub const A3: Register = Register(13);
    pub const A4: Register = Register(14);
    pub const A5: Register = Register(15);
    pub const A6: Register = Register(16);
    pub const A7: Register = Register(17);
    pub const S2: Register = Register(18);
    pub const S3: Register = Register(19);
    pub const S4: Register = Register(20);
    pub const S5: Register = Register(21);
    pub const S6: Register = Register(22);
    pub const S7: Register = Register(23);
    pub const S8: Register = Register(24);
    pub const S9: Register = Register(25);
    pub const S10: Register = Register(26);
    pub const S11: Register = Register(27);
    pub const T3: Register = Register(28);
    pub const T4: Register = Register(29);
    pub const T5: Register = Register(30);
    pub const T6: Register = Register(31);

    pub fn new(val: u8) -> Self {
        assert!(val < 32, "Register index out of range: {}", val);
        Register(val)
    }

    pub fn index(self) -> usize {
        self.0 as usize
    }
}

impl fmt::Display for Register {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.0 {
            0 => write!(f, "zero"),
            1 => write!(f, "ra"),
            2 => write!(f, "sp"),
            3 => write!(f, "gp"),
            4 => write!(f, "tp"),
            5..=7 => write!(f, "t{}", self.0 - 5),
            8 => write!(f, "s0"),
            9 => write!(f, "s1"),
            10..=17 => write!(f, "a{}", self.0 - 10),
            18..=27 => write!(f, "s{}", self.0 - 16),
            28..=31 => write!(f, "t{}", self.0 - 25),
            _ => write!(f, "x{}", self.0),
        }
    }
}

/// RISC-V rv32im instruction
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Instruction {
    // RV32I Base Integer Instructions

    // R-type instructions
    Add { rd: Register, rs1: Register, rs2: Register },
    Sub { rd: Register, rs1: Register, rs2: Register },
    Xor { rd: Register, rs1: Register, rs2: Register },
    Or { rd: Register, rs1: Register, rs2: Register },
    And { rd: Register, rs1: Register, rs2: Register },
    Sll { rd: Register, rs1: Register, rs2: Register },
    Srl { rd: Register, rs1: Register, rs2: Register },
    Sra { rd: Register, rs1: Register, rs2: Register },
    Slt { rd: Register, rs1: Register, rs2: Register },
    Sltu { rd: Register, rs1: Register, rs2: Register },

    // I-type instructions
    Addi { rd: Register, rs1: Register, imm: i32 },
    Xori { rd: Register, rs1: Register, imm: i32 },
    Ori { rd: Register, rs1: Register, imm: i32 },
    Andi { rd: Register, rs1: Register, imm: i32 },
    Slli { rd: Register, rs1: Register, shamt: u32 },
    Srli { rd: Register, rs1: Register, shamt: u32 },
    Srai { rd: Register, rs1: Register, shamt: u32 },
    Slti { rd: Register, rs1: Register, imm: i32 },
    Sltiu { rd: Register, rs1: Register, imm: i32 },

    // Load instructions (I-type)
    Lb { rd: Register, rs1: Register, imm: i32 },
    Lh { rd: Register, rs1: Register, imm: i32 },
    Lw { rd: Register, rs1: Register, imm: i32 },
    Lbu { rd: Register, rs1: Register, imm: i32 },
    Lhu { rd: Register, rs1: Register, imm: i32 },

    // S-type instructions (Store)
    Sb { rs1: Register, rs2: Register, imm: i32 },
    Sh { rs1: Register, rs2: Register, imm: i32 },
    Sw { rs1: Register, rs2: Register, imm: i32 },

    // B-type instructions (Branch)
    Beq { rs1: Register, rs2: Register, imm: i32 },
    Bne { rs1: Register, rs2: Register, imm: i32 },
    Blt { rs1: Register, rs2: Register, imm: i32 },
    Bge { rs1: Register, rs2: Register, imm: i32 },
    Bltu { rs1: Register, rs2: Register, imm: i32 },
    Bgeu { rs1: Register, rs2: Register, imm: i32 },

    // J-type instructions
    Jal { rd: Register, imm: i32 },

    // JALR (I-type)
    Jalr { rd: Register, rs1: Register, imm: i32 },

    // U-type instructions
    Lui { rd: Register, imm: i32 },
    Auipc { rd: Register, imm: i32 },

    // System instructions
    Ecall,
    Ebreak,
    Fence { pred: u8, succ: u8 },
    FenceI,

    // CSR instructions
    Csrrw { rd: Register, rs1: Register, csr: u16 },
    Csrrs { rd: Register, rs1: Register, csr: u16 },
    Csrrc { rd: Register, rs1: Register, csr: u16 },
    Csrrwi { rd: Register, uimm: u8, csr: u16 },
    Csrrsi { rd: Register, uimm: u8, csr: u16 },
    Csrrci { rd: Register, uimm: u8, csr: u16 },

    // RV32M Standard Extension (Multiply/Divide)
    Mul { rd: Register, rs1: Register, rs2: Register },
    Mulh { rd: Register, rs1: Register, rs2: Register },
    Mulhsu { rd: Register, rs1: Register, rs2: Register },
    Mulhu { rd: Register, rs1: Register, rs2: Register },
    Div { rd: Register, rs1: Register, rs2: Register },
    Divu { rd: Register, rs1: Register, rs2: Register },
    Rem { rd: Register, rs1: Register, rs2: Register },
    Remu { rd: Register, rs1: Register, rs2: Register },

    // Pseudo-instruction for unknown/invalid
    Unknown(u32),
}

impl Instruction {
    /// Returns the destination register if any
    pub fn rd(&self) -> Option<Register> {
        use Instruction::*;
        match self {
            Add { rd, .. } | Sub { rd, .. } | Xor { rd, .. } | Or { rd, .. } | And { rd, .. } |
            Sll { rd, .. } | Srl { rd, .. } | Sra { rd, .. } | Slt { rd, .. } | Sltu { rd, .. } |
            Addi { rd, .. } | Xori { rd, .. } | Ori { rd, .. } | Andi { rd, .. } |
            Slli { rd, .. } | Srli { rd, .. } | Srai { rd, .. } | Slti { rd, .. } | Sltiu { rd, .. } |
            Lb { rd, .. } | Lh { rd, .. } | Lw { rd, .. } | Lbu { rd, .. } | Lhu { rd, .. } |
            Jal { rd, .. } | Jalr { rd, .. } | Lui { rd, .. } | Auipc { rd, .. } |
            Csrrw { rd, .. } | Csrrs { rd, .. } | Csrrc { rd, .. } |
            Csrrwi { rd, .. } | Csrrsi { rd, .. } | Csrrci { rd, .. } |
            Mul { rd, .. } | Mulh { rd, .. } | Mulhsu { rd, .. } | Mulhu { rd, .. } |
            Div { rd, .. } | Divu { rd, .. } | Rem { rd, .. } | Remu { rd, .. } => Some(*rd),
            _ => None,
        }
    }

    /// Returns true if this is a branch or jump instruction
    pub fn is_control_flow(&self) -> bool {
        use Instruction::*;
        matches!(self,
            Beq { .. } | Bne { .. } | Blt { .. } | Bge { .. } | Bltu { .. } | Bgeu { .. } |
            Jal { .. } | Jalr { .. } | Ecall | Ebreak
        )
    }
}

impl fmt::Display for Instruction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use Instruction::*;
        match self {
            Add { rd, rs1, rs2 } => write!(f, "add {}, {}, {}", rd, rs1, rs2),
            Sub { rd, rs1, rs2 } => write!(f, "sub {}, {}, {}", rd, rs1, rs2),
            Xor { rd, rs1, rs2 } => write!(f, "xor {}, {}, {}", rd, rs1, rs2),
            Or { rd, rs1, rs2 } => write!(f, "or {}, {}, {}", rd, rs1, rs2),
            And { rd, rs1, rs2 } => write!(f, "and {}, {}, {}", rd, rs1, rs2),
            Sll { rd, rs1, rs2 } => write!(f, "sll {}, {}, {}", rd, rs1, rs2),
            Srl { rd, rs1, rs2 } => write!(f, "srl {}, {}, {}", rd, rs1, rs2),
            Sra { rd, rs1, rs2 } => write!(f, "sra {}, {}, {}", rd, rs1, rs2),
            Slt { rd, rs1, rs2 } => write!(f, "slt {}, {}, {}", rd, rs1, rs2),
            Sltu { rd, rs1, rs2 } => write!(f, "sltu {}, {}, {}", rd, rs1, rs2),
            Addi { rd, rs1, imm } => write!(f, "addi {}, {}, {}", rd, rs1, imm),
            Xori { rd, rs1, imm } => write!(f, "xori {}, {}, {}", rd, rs1, imm),
            Ori { rd, rs1, imm } => write!(f, "ori {}, {}, {}", rd, rs1, imm),
            Andi { rd, rs1, imm } => write!(f, "andi {}, {}, {}", rd, rs1, imm),
            Slli { rd, rs1, shamt } => write!(f, "slli {}, {}, {}", rd, rs1, shamt),
            Srli { rd, rs1, shamt } => write!(f, "srli {}, {}, {}", rd, rs1, shamt),
            Srai { rd, rs1, shamt } => write!(f, "srai {}, {}, {}", rd, rs1, shamt),
            Slti { rd, rs1, imm } => write!(f, "slti {}, {}, {}", rd, rs1, imm),
            Sltiu { rd, rs1, imm } => write!(f, "sltiu {}, {}, {}", rd, rs1, imm),
            Lb { rd, rs1, imm } => write!(f, "lb {}, {}({})", rd, imm, rs1),
            Lh { rd, rs1, imm } => write!(f, "lh {}, {}({})", rd, imm, rs1),
            Lw { rd, rs1, imm } => write!(f, "lw {}, {}({})", rd, imm, rs1),
            Lbu { rd, rs1, imm } => write!(f, "lbu {}, {}({})", rd, imm, rs1),
            Lhu { rd, rs1, imm } => write!(f, "lhu {}, {}({})", rd, imm, rs1),
            Sb { rs1, rs2, imm } => write!(f, "sb {}, {}({})", rs2, imm, rs1),
            Sh { rs1, rs2, imm } => write!(f, "sh {}, {}({})", rs2, imm, rs1),
            Sw { rs1, rs2, imm } => write!(f, "sw {}, {}({})", rs2, imm, rs1),
            Beq { rs1, rs2, imm } => write!(f, "beq {}, {}, {}", rs1, rs2, imm),
            Bne { rs1, rs2, imm } => write!(f, "bne {}, {}, {}", rs1, rs2, imm),
            Blt { rs1, rs2, imm } => write!(f, "blt {}, {}, {}", rs1, rs2, imm),
            Bge { rs1, rs2, imm } => write!(f, "bge {}, {}, {}", rs1, rs2, imm),
            Bltu { rs1, rs2, imm } => write!(f, "bltu {}, {}, {}", rs1, rs2, imm),
            Bgeu { rs1, rs2, imm } => write!(f, "bgeu {}, {}, {}", rs1, rs2, imm),
            Jal { rd, imm } => write!(f, "jal {}, {}", rd, imm),
            Jalr { rd, rs1, imm } => write!(f, "jalr {}, {}({})", rd, imm, rs1),
            Lui { rd, imm } => write!(f, "lui {}, {}", rd, imm),
            Auipc { rd, imm } => write!(f, "auipc {}, {}", rd, imm),
            Ecall => write!(f, "ecall"),
            Ebreak => write!(f, "ebreak"),
            Fence { pred, succ } => write!(f, "fence {}, {}", pred, succ),
            FenceI => write!(f, "fence.i"),
            Csrrw { rd, rs1, csr } => write!(f, "csrrw {}, {:#x}, {}", rd, csr, rs1),
            Csrrs { rd, rs1, csr } => write!(f, "csrrs {}, {:#x}, {}", rd, csr, rs1),
            Csrrc { rd, rs1, csr } => write!(f, "csrrc {}, {:#x}, {}", rd, csr, rs1),
            Csrrwi { rd, uimm, csr } => write!(f, "csrrwi {}, {:#x}, {}", rd, csr, uimm),
            Csrrsi { rd, uimm, csr } => write!(f, "csrrsi {}, {:#x}, {}", rd, csr, uimm),
            Csrrci { rd, uimm, csr } => write!(f, "csrrci {}, {:#x}, {}", rd, csr, uimm),
            Mul { rd, rs1, rs2 } => write!(f, "mul {}, {}, {}", rd, rs1, rs2),
            Mulh { rd, rs1, rs2 } => write!(f, "mulh {}, {}, {}", rd, rs1, rs2),
            Mulhsu { rd, rs1, rs2 } => write!(f, "mulhsu {}, {}, {}", rd, rs1, rs2),
            Mulhu { rd, rs1, rs2 } => write!(f, "mulhu {}, {}, {}", rd, rs1, rs2),
            Div { rd, rs1, rs2 } => write!(f, "div {}, {}, {}", rd, rs1, rs2),
            Divu { rd, rs1, rs2 } => write!(f, "divu {}, {}, {}", rd, rs1, rs2),
            Rem { rd, rs1, rs2 } => write!(f, "rem {}, {}, {}", rd, rs1, rs2),
            Remu { rd, rs1, rs2 } => write!(f, "remu {}, {}, {}", rd, rs1, rs2),
            Unknown(raw) => write!(f, ".word {:#010x}", raw),
        }
    }
}
