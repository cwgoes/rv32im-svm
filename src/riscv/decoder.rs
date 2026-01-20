//! RISC-V rv32im instruction decoder

use super::instruction::{Instruction, Register};

/// Decode a 32-bit RISC-V instruction
pub fn decode(word: u32) -> Instruction {
    let opcode = word & 0x7f;
    let rd = Register::new(((word >> 7) & 0x1f) as u8);
    let funct3 = (word >> 12) & 0x7;
    let rs1 = Register::new(((word >> 15) & 0x1f) as u8);
    let rs2 = Register::new(((word >> 20) & 0x1f) as u8);
    let funct7 = (word >> 25) & 0x7f;

    match opcode {
        // R-type: OP
        0b0110011 => decode_r_type(rd, funct3, rs1, rs2, funct7, word),

        // I-type: OP-IMM
        0b0010011 => decode_i_type_alu(rd, funct3, rs1, word),

        // I-type: LOAD
        0b0000011 => decode_load(rd, funct3, rs1, word),

        // S-type: STORE
        0b0100011 => decode_store(funct3, rs1, rs2, word),

        // B-type: BRANCH
        0b1100011 => decode_branch(funct3, rs1, rs2, word),

        // J-type: JAL
        0b1101111 => {
            let imm = decode_j_imm(word);
            Instruction::Jal { rd, imm }
        }

        // I-type: JALR
        0b1100111 => {
            let imm = decode_i_imm(word);
            if funct3 == 0 {
                Instruction::Jalr { rd, rs1, imm }
            } else {
                Instruction::Unknown(word)
            }
        }

        // U-type: LUI
        0b0110111 => {
            let imm = decode_u_imm(word);
            Instruction::Lui { rd, imm }
        }

        // U-type: AUIPC
        0b0010111 => {
            let imm = decode_u_imm(word);
            Instruction::Auipc { rd, imm }
        }

        // SYSTEM
        0b1110011 => decode_system(rd, funct3, rs1, word),

        // MISC-MEM (FENCE)
        0b0001111 => {
            if funct3 == 0 {
                let pred = ((word >> 24) & 0xf) as u8;
                let succ = ((word >> 20) & 0xf) as u8;
                Instruction::Fence { pred, succ }
            } else if funct3 == 1 {
                Instruction::FenceI
            } else {
                Instruction::Unknown(word)
            }
        }

        _ => Instruction::Unknown(word),
    }
}

fn decode_r_type(rd: Register, funct3: u32, rs1: Register, rs2: Register, funct7: u32, word: u32) -> Instruction {
    match (funct7, funct3) {
        // RV32I
        (0b0000000, 0b000) => Instruction::Add { rd, rs1, rs2 },
        (0b0100000, 0b000) => Instruction::Sub { rd, rs1, rs2 },
        (0b0000000, 0b001) => Instruction::Sll { rd, rs1, rs2 },
        (0b0000000, 0b010) => Instruction::Slt { rd, rs1, rs2 },
        (0b0000000, 0b011) => Instruction::Sltu { rd, rs1, rs2 },
        (0b0000000, 0b100) => Instruction::Xor { rd, rs1, rs2 },
        (0b0000000, 0b101) => Instruction::Srl { rd, rs1, rs2 },
        (0b0100000, 0b101) => Instruction::Sra { rd, rs1, rs2 },
        (0b0000000, 0b110) => Instruction::Or { rd, rs1, rs2 },
        (0b0000000, 0b111) => Instruction::And { rd, rs1, rs2 },

        // RV32M
        (0b0000001, 0b000) => Instruction::Mul { rd, rs1, rs2 },
        (0b0000001, 0b001) => Instruction::Mulh { rd, rs1, rs2 },
        (0b0000001, 0b010) => Instruction::Mulhsu { rd, rs1, rs2 },
        (0b0000001, 0b011) => Instruction::Mulhu { rd, rs1, rs2 },
        (0b0000001, 0b100) => Instruction::Div { rd, rs1, rs2 },
        (0b0000001, 0b101) => Instruction::Divu { rd, rs1, rs2 },
        (0b0000001, 0b110) => Instruction::Rem { rd, rs1, rs2 },
        (0b0000001, 0b111) => Instruction::Remu { rd, rs1, rs2 },

        _ => Instruction::Unknown(word),
    }
}

fn decode_i_type_alu(rd: Register, funct3: u32, rs1: Register, word: u32) -> Instruction {
    let imm = decode_i_imm(word);
    let shamt = ((word >> 20) & 0x1f) as u32;
    let funct7 = (word >> 25) & 0x7f;

    match funct3 {
        0b000 => Instruction::Addi { rd, rs1, imm },
        0b010 => Instruction::Slti { rd, rs1, imm },
        0b011 => Instruction::Sltiu { rd, rs1, imm },
        0b100 => Instruction::Xori { rd, rs1, imm },
        0b110 => Instruction::Ori { rd, rs1, imm },
        0b111 => Instruction::Andi { rd, rs1, imm },
        0b001 => {
            if funct7 == 0 {
                Instruction::Slli { rd, rs1, shamt }
            } else {
                Instruction::Unknown(word)
            }
        }
        0b101 => {
            match funct7 {
                0b0000000 => Instruction::Srli { rd, rs1, shamt },
                0b0100000 => Instruction::Srai { rd, rs1, shamt },
                _ => Instruction::Unknown(word),
            }
        }
        _ => Instruction::Unknown(word),
    }
}

fn decode_load(rd: Register, funct3: u32, rs1: Register, word: u32) -> Instruction {
    let imm = decode_i_imm(word);

    match funct3 {
        0b000 => Instruction::Lb { rd, rs1, imm },
        0b001 => Instruction::Lh { rd, rs1, imm },
        0b010 => Instruction::Lw { rd, rs1, imm },
        0b100 => Instruction::Lbu { rd, rs1, imm },
        0b101 => Instruction::Lhu { rd, rs1, imm },
        _ => Instruction::Unknown(word),
    }
}

fn decode_store(funct3: u32, rs1: Register, rs2: Register, word: u32) -> Instruction {
    let imm = decode_s_imm(word);

    match funct3 {
        0b000 => Instruction::Sb { rs1, rs2, imm },
        0b001 => Instruction::Sh { rs1, rs2, imm },
        0b010 => Instruction::Sw { rs1, rs2, imm },
        _ => Instruction::Unknown(word),
    }
}

fn decode_branch(funct3: u32, rs1: Register, rs2: Register, word: u32) -> Instruction {
    let imm = decode_b_imm(word);

    match funct3 {
        0b000 => Instruction::Beq { rs1, rs2, imm },
        0b001 => Instruction::Bne { rs1, rs2, imm },
        0b100 => Instruction::Blt { rs1, rs2, imm },
        0b101 => Instruction::Bge { rs1, rs2, imm },
        0b110 => Instruction::Bltu { rs1, rs2, imm },
        0b111 => Instruction::Bgeu { rs1, rs2, imm },
        _ => Instruction::Unknown(word),
    }
}

fn decode_system(rd: Register, funct3: u32, rs1: Register, word: u32) -> Instruction {
    let csr = ((word >> 20) & 0xfff) as u16;
    let uimm = ((word >> 15) & 0x1f) as u8;

    match funct3 {
        0b000 => {
            let imm = (word >> 20) & 0xfff;
            match imm {
                0 => Instruction::Ecall,
                1 => Instruction::Ebreak,
                _ => Instruction::Unknown(word),
            }
        }
        0b001 => Instruction::Csrrw { rd, rs1, csr },
        0b010 => Instruction::Csrrs { rd, rs1, csr },
        0b011 => Instruction::Csrrc { rd, rs1, csr },
        0b101 => Instruction::Csrrwi { rd, uimm, csr },
        0b110 => Instruction::Csrrsi { rd, uimm, csr },
        0b111 => Instruction::Csrrci { rd, uimm, csr },
        _ => Instruction::Unknown(word),
    }
}

// Immediate decoding functions

fn decode_i_imm(word: u32) -> i32 {
    let imm = (word >> 20) & 0xfff;
    // Sign extend from 12 bits
    if imm & 0x800 != 0 {
        (imm | 0xfffff000) as i32
    } else {
        imm as i32
    }
}

fn decode_s_imm(word: u32) -> i32 {
    let imm11_5 = (word >> 25) & 0x7f;
    let imm4_0 = (word >> 7) & 0x1f;
    let imm = (imm11_5 << 5) | imm4_0;
    // Sign extend from 12 bits
    if imm & 0x800 != 0 {
        (imm | 0xfffff000) as i32
    } else {
        imm as i32
    }
}

fn decode_b_imm(word: u32) -> i32 {
    let imm12 = (word >> 31) & 0x1;
    let imm10_5 = (word >> 25) & 0x3f;
    let imm4_1 = (word >> 8) & 0xf;
    let imm11 = (word >> 7) & 0x1;
    let imm = (imm12 << 12) | (imm11 << 11) | (imm10_5 << 5) | (imm4_1 << 1);
    // Sign extend from 13 bits
    if imm & 0x1000 != 0 {
        (imm | 0xffffe000) as i32
    } else {
        imm as i32
    }
}

fn decode_u_imm(word: u32) -> i32 {
    (word & 0xfffff000) as i32
}

fn decode_j_imm(word: u32) -> i32 {
    let imm20 = (word >> 31) & 0x1;
    let imm10_1 = (word >> 21) & 0x3ff;
    let imm11 = (word >> 20) & 0x1;
    let imm19_12 = (word >> 12) & 0xff;
    let imm = (imm20 << 20) | (imm19_12 << 12) | (imm11 << 11) | (imm10_1 << 1);
    // Sign extend from 21 bits
    if imm & 0x100000 != 0 {
        (imm | 0xffe00000) as i32
    } else {
        imm as i32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode_add() {
        // add x1, x2, x3
        let word = 0x003100b3;
        let inst = decode(word);
        assert_eq!(inst, Instruction::Add {
            rd: Register::new(1),
            rs1: Register::new(2),
            rs2: Register::new(3),
        });
    }

    #[test]
    fn test_decode_addi() {
        // addi x1, x2, 10
        let word = 0x00a10093;
        let inst = decode(word);
        assert_eq!(inst, Instruction::Addi {
            rd: Register::new(1),
            rs1: Register::new(2),
            imm: 10,
        });
    }

    #[test]
    fn test_decode_negative_imm() {
        // addi x1, x0, -1
        let word = 0xfff00093;
        let inst = decode(word);
        assert_eq!(inst, Instruction::Addi {
            rd: Register::new(1),
            rs1: Register::new(0),
            imm: -1,
        });
    }

    #[test]
    fn test_decode_lui() {
        // lui x1, 0x12345
        let word = 0x123450b7;
        let inst = decode(word);
        assert_eq!(inst, Instruction::Lui {
            rd: Register::new(1),
            imm: 0x12345000_u32 as i32,
        });
    }

    #[test]
    fn test_decode_jal() {
        // jal x1, 0 (encoded)
        let word = 0x000000ef;
        let inst = decode(word);
        assert_eq!(inst, Instruction::Jal {
            rd: Register::new(1),
            imm: 0,
        });
    }

    #[test]
    fn test_decode_beq() {
        // beq x1, x2, 8
        let word = 0x00208463;
        let inst = decode(word);
        assert_eq!(inst, Instruction::Beq {
            rs1: Register::new(1),
            rs2: Register::new(2),
            imm: 8,
        });
    }

    #[test]
    fn test_decode_mul() {
        // mul x1, x2, x3
        let word = 0x023100b3;
        let inst = decode(word);
        assert_eq!(inst, Instruction::Mul {
            rd: Register::new(1),
            rs1: Register::new(2),
            rs2: Register::new(3),
        });
    }
}
