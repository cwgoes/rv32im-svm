//! RISC-V rv32i instruction tests
//!
//! These tests are based on the official RISC-V test suite patterns.

use rv32im_svm::{compile, execute, Instruction, Register};

// Helper to run a program and get the result
fn run_program(instructions: Vec<Instruction>) -> u64 {
    let program = compile(&instructions).expect("Compilation failed");
    execute(&program).expect("Execution failed")
}

// Helper macro for creating test cases
macro_rules! test_r_type {
    ($name:ident, $inst:ident, $rd:expr, $rs1_val:expr, $rs2_val:expr, $expected:expr) => {
        #[test]
        fn $name() {
            let instructions = vec![
                Instruction::Addi { rd: Register::A1, rs1: Register::ZERO, imm: $rs1_val },
                Instruction::Addi { rd: Register::A2, rs1: Register::ZERO, imm: $rs2_val },
                Instruction::$inst { rd: Register::A0, rs1: Register::A1, rs2: Register::A2 },
                Instruction::Ecall,
            ];
            let result = run_program(instructions);
            assert_eq!(result as i64, $expected as i64, "Expected {}, got {}", $expected, result as i64);
        }
    };
}

macro_rules! test_i_type {
    ($name:ident, $inst:ident, $rs1_val:expr, $imm:expr, $expected:expr) => {
        #[test]
        fn $name() {
            let instructions = vec![
                Instruction::Addi { rd: Register::A1, rs1: Register::ZERO, imm: $rs1_val },
                Instruction::$inst { rd: Register::A0, rs1: Register::A1, imm: $imm },
                Instruction::Ecall,
            ];
            let result = run_program(instructions);
            assert_eq!(result as i64, $expected as i64, "Expected {}, got {}", $expected, result as i64);
        }
    };
}

// ============================================================================
// ADD tests
// ============================================================================

test_r_type!(test_add_positive, Add, a0, 10, 20, 30);
test_r_type!(test_add_zero, Add, a0, 0, 0, 0);
test_r_type!(test_add_negative, Add, a0, -10, -20, -30);
test_r_type!(test_add_mixed, Add, a0, 100, -50, 50);

#[test]
fn test_add_overflow() {
    // Test overflow (wraps around)
    // Load 0x7FFFFFFF (INT_MAX) using LUI + ADDI
    // LUI 0x80000000 + ADDI -1 = 0x80000000 - 1 = 0x7FFFFFFF
    let instructions = vec![
        Instruction::Lui { rd: Register::A1, imm: 0x80000000_u32 as i32 },
        Instruction::Addi { rd: Register::A1, rs1: Register::A1, imm: -1 },
        Instruction::Addi { rd: Register::A2, rs1: Register::ZERO, imm: 1 },
        Instruction::Add { rd: Register::A0, rs1: Register::A1, rs2: Register::A2 },
        Instruction::Ecall,
    ];
    let result = run_program(instructions) as i32;
    assert_eq!(result, i32::MIN);
}

// ============================================================================
// SUB tests
// ============================================================================

test_r_type!(test_sub_positive, Sub, a0, 30, 10, 20);
test_r_type!(test_sub_zero, Sub, a0, 10, 10, 0);
test_r_type!(test_sub_negative_result, Sub, a0, 10, 30, -20);

// ============================================================================
// ADDI tests
// ============================================================================

test_i_type!(test_addi_positive, Addi, 10, 20, 30);
test_i_type!(test_addi_zero, Addi, 0, 0, 0);
test_i_type!(test_addi_negative, Addi, 10, -5, 5);
test_i_type!(test_addi_max_imm, Addi, 0, 2047, 2047);
test_i_type!(test_addi_min_imm, Addi, 0, -2048, -2048);

// ============================================================================
// AND/ANDI tests
// ============================================================================

test_r_type!(test_and_all_ones, And, a0, -1, -1, -1);
test_r_type!(test_and_zero, And, a0, 0xFF, 0, 0);
test_r_type!(test_and_partial, And, a0, 0xFF, 0x0F, 0x0F);

test_i_type!(test_andi_basic, Andi, 0xFF, 0x0F, 0x0F);
test_i_type!(test_andi_zero, Andi, 0xFF, 0, 0);

// ============================================================================
// OR/ORI tests
// ============================================================================

test_r_type!(test_or_basic, Or, a0, 0xF0, 0x0F, 0xFF);
test_r_type!(test_or_same, Or, a0, 0xAA, 0xAA, 0xAA);
test_r_type!(test_or_zero, Or, a0, 0, 0, 0);

test_i_type!(test_ori_basic, Ori, 0xF0, 0x0F, 0xFF);

// ============================================================================
// XOR/XORI tests
// ============================================================================

test_r_type!(test_xor_basic, Xor, a0, 0xFF, 0x0F, 0xF0);
test_r_type!(test_xor_same, Xor, a0, 0xAA, 0xAA, 0);

test_i_type!(test_xori_basic, Xori, 0xFF, 0x0F, 0xF0);

// ============================================================================
// Shift tests (SLL, SRL, SRA, SLLI, SRLI, SRAI)
// ============================================================================

#[test]
fn test_slli_basic() {
    let instructions = vec![
        Instruction::Addi { rd: Register::A1, rs1: Register::ZERO, imm: 1 },
        Instruction::Slli { rd: Register::A0, rs1: Register::A1, shamt: 4 },
        Instruction::Ecall,
    ];
    assert_eq!(run_program(instructions), 16);
}

#[test]
fn test_slli_zero_shift() {
    let instructions = vec![
        Instruction::Addi { rd: Register::A1, rs1: Register::ZERO, imm: 0x55 },
        Instruction::Slli { rd: Register::A0, rs1: Register::A1, shamt: 0 },
        Instruction::Ecall,
    ];
    assert_eq!(run_program(instructions), 0x55);
}

#[test]
fn test_srli_basic() {
    let instructions = vec![
        Instruction::Addi { rd: Register::A1, rs1: Register::ZERO, imm: 0x100 },
        Instruction::Srli { rd: Register::A0, rs1: Register::A1, shamt: 4 },
        Instruction::Ecall,
    ];
    assert_eq!(run_program(instructions), 0x10);
}

#[test]
fn test_srai_positive() {
    let instructions = vec![
        Instruction::Addi { rd: Register::A1, rs1: Register::ZERO, imm: 0x100 },
        Instruction::Srai { rd: Register::A0, rs1: Register::A1, shamt: 4 },
        Instruction::Ecall,
    ];
    assert_eq!(run_program(instructions), 0x10);
}

#[test]
fn test_srai_negative() {
    let instructions = vec![
        Instruction::Addi { rd: Register::A1, rs1: Register::ZERO, imm: -16 },
        Instruction::Srai { rd: Register::A0, rs1: Register::A1, shamt: 2 },
        Instruction::Ecall,
    ];
    // -16 >> 2 = -4 (arithmetic shift preserves sign)
    assert_eq!(run_program(instructions) as i32, -4);
}

#[test]
fn test_sll_register() {
    let instructions = vec![
        Instruction::Addi { rd: Register::A1, rs1: Register::ZERO, imm: 1 },
        Instruction::Addi { rd: Register::A2, rs1: Register::ZERO, imm: 3 },
        Instruction::Sll { rd: Register::A0, rs1: Register::A1, rs2: Register::A2 },
        Instruction::Ecall,
    ];
    assert_eq!(run_program(instructions), 8);
}

#[test]
fn test_srl_register() {
    let instructions = vec![
        Instruction::Addi { rd: Register::A1, rs1: Register::ZERO, imm: 16 },
        Instruction::Addi { rd: Register::A2, rs1: Register::ZERO, imm: 2 },
        Instruction::Srl { rd: Register::A0, rs1: Register::A1, rs2: Register::A2 },
        Instruction::Ecall,
    ];
    assert_eq!(run_program(instructions), 4);
}

#[test]
fn test_sra_register_negative() {
    let instructions = vec![
        Instruction::Addi { rd: Register::A1, rs1: Register::ZERO, imm: -32 },
        Instruction::Addi { rd: Register::A2, rs1: Register::ZERO, imm: 3 },
        Instruction::Sra { rd: Register::A0, rs1: Register::A1, rs2: Register::A2 },
        Instruction::Ecall,
    ];
    assert_eq!(run_program(instructions) as i32, -4);
}

// ============================================================================
// SLT/SLTU/SLTI/SLTIU tests
// ============================================================================

#[test]
fn test_slt_less_than() {
    let instructions = vec![
        Instruction::Addi { rd: Register::A1, rs1: Register::ZERO, imm: 5 },
        Instruction::Addi { rd: Register::A2, rs1: Register::ZERO, imm: 10 },
        Instruction::Slt { rd: Register::A0, rs1: Register::A1, rs2: Register::A2 },
        Instruction::Ecall,
    ];
    assert_eq!(run_program(instructions), 1);
}

#[test]
fn test_slt_not_less_than() {
    let instructions = vec![
        Instruction::Addi { rd: Register::A1, rs1: Register::ZERO, imm: 10 },
        Instruction::Addi { rd: Register::A2, rs1: Register::ZERO, imm: 5 },
        Instruction::Slt { rd: Register::A0, rs1: Register::A1, rs2: Register::A2 },
        Instruction::Ecall,
    ];
    assert_eq!(run_program(instructions), 0);
}

#[test]
fn test_slt_equal() {
    let instructions = vec![
        Instruction::Addi { rd: Register::A1, rs1: Register::ZERO, imm: 5 },
        Instruction::Addi { rd: Register::A2, rs1: Register::ZERO, imm: 5 },
        Instruction::Slt { rd: Register::A0, rs1: Register::A1, rs2: Register::A2 },
        Instruction::Ecall,
    ];
    assert_eq!(run_program(instructions), 0);
}

#[test]
fn test_slt_negative() {
    let instructions = vec![
        Instruction::Addi { rd: Register::A1, rs1: Register::ZERO, imm: -5 },
        Instruction::Addi { rd: Register::A2, rs1: Register::ZERO, imm: 5 },
        Instruction::Slt { rd: Register::A0, rs1: Register::A1, rs2: Register::A2 },
        Instruction::Ecall,
    ];
    assert_eq!(run_program(instructions), 1);
}

#[test]
fn test_sltu_unsigned() {
    let instructions = vec![
        Instruction::Addi { rd: Register::A1, rs1: Register::ZERO, imm: -1 }, // Large unsigned
        Instruction::Addi { rd: Register::A2, rs1: Register::ZERO, imm: 1 },
        Instruction::Sltu { rd: Register::A0, rs1: Register::A1, rs2: Register::A2 },
        Instruction::Ecall,
    ];
    // -1 as unsigned is larger than 1
    assert_eq!(run_program(instructions), 0);
}

#[test]
fn test_slti_basic() {
    let instructions = vec![
        Instruction::Addi { rd: Register::A1, rs1: Register::ZERO, imm: 5 },
        Instruction::Slti { rd: Register::A0, rs1: Register::A1, imm: 10 },
        Instruction::Ecall,
    ];
    assert_eq!(run_program(instructions), 1);
}

#[test]
fn test_sltiu_basic() {
    let instructions = vec![
        Instruction::Addi { rd: Register::A1, rs1: Register::ZERO, imm: 5 },
        Instruction::Sltiu { rd: Register::A0, rs1: Register::A1, imm: 10 },
        Instruction::Ecall,
    ];
    assert_eq!(run_program(instructions), 1);
}

// ============================================================================
// LUI/AUIPC tests
// ============================================================================

#[test]
fn test_lui_basic() {
    let instructions = vec![
        Instruction::Lui { rd: Register::A0, imm: 0x12345000_u32 as i32 },
        Instruction::Ecall,
    ];
    assert_eq!(run_program(instructions) as u32, 0x12345000);
}

#[test]
fn test_lui_max() {
    let instructions = vec![
        Instruction::Lui { rd: Register::A0, imm: 0xfffff000_u32 as i32 },
        Instruction::Ecall,
    ];
    // Sign extended
    assert_eq!(run_program(instructions) as i32, -4096);
}

#[test]
fn test_auipc_basic() {
    // auipc at PC=0 should just load the immediate
    let instructions = vec![
        Instruction::Auipc { rd: Register::A0, imm: 0x1000 },
        Instruction::Ecall,
    ];
    let result = run_program(instructions) as u32;
    // PC is 0, so result = 0 + 0x1000 = 0x1000
    assert_eq!(result, 0x1000);
}

// ============================================================================
// Branch tests (BEQ, BNE, BLT, BGE, BLTU, BGEU)
// ============================================================================

#[test]
fn test_beq_taken() {
    let instructions = vec![
        Instruction::Addi { rd: Register::A1, rs1: Register::ZERO, imm: 5 },
        Instruction::Addi { rd: Register::A2, rs1: Register::ZERO, imm: 5 },
        Instruction::Beq { rs1: Register::A1, rs2: Register::A2, imm: 8 }, // Skip next
        Instruction::Addi { rd: Register::A0, rs1: Register::ZERO, imm: 0 }, // Skipped
        Instruction::Addi { rd: Register::A0, rs1: Register::ZERO, imm: 1 },
        Instruction::Ecall,
    ];
    assert_eq!(run_program(instructions), 1);
}

#[test]
fn test_beq_not_taken() {
    let instructions = vec![
        Instruction::Addi { rd: Register::A1, rs1: Register::ZERO, imm: 5 },
        Instruction::Addi { rd: Register::A2, rs1: Register::ZERO, imm: 6 },
        Instruction::Beq { rs1: Register::A1, rs2: Register::A2, imm: 8 },
        Instruction::Addi { rd: Register::A0, rs1: Register::ZERO, imm: 1 }, // Not skipped
        Instruction::Ecall,
    ];
    assert_eq!(run_program(instructions), 1);
}

#[test]
fn test_bne_taken() {
    let instructions = vec![
        Instruction::Addi { rd: Register::A1, rs1: Register::ZERO, imm: 5 },
        Instruction::Addi { rd: Register::A2, rs1: Register::ZERO, imm: 6 },
        Instruction::Bne { rs1: Register::A1, rs2: Register::A2, imm: 8 },
        Instruction::Addi { rd: Register::A0, rs1: Register::ZERO, imm: 0 },
        Instruction::Addi { rd: Register::A0, rs1: Register::ZERO, imm: 1 },
        Instruction::Ecall,
    ];
    assert_eq!(run_program(instructions), 1);
}

#[test]
fn test_blt_taken() {
    let instructions = vec![
        Instruction::Addi { rd: Register::A1, rs1: Register::ZERO, imm: 5 },
        Instruction::Addi { rd: Register::A2, rs1: Register::ZERO, imm: 10 },
        Instruction::Blt { rs1: Register::A1, rs2: Register::A2, imm: 8 },
        Instruction::Addi { rd: Register::A0, rs1: Register::ZERO, imm: 0 },
        Instruction::Addi { rd: Register::A0, rs1: Register::ZERO, imm: 1 },
        Instruction::Ecall,
    ];
    assert_eq!(run_program(instructions), 1);
}

#[test]
fn test_blt_negative() {
    let instructions = vec![
        Instruction::Addi { rd: Register::A1, rs1: Register::ZERO, imm: -5 },
        Instruction::Addi { rd: Register::A2, rs1: Register::ZERO, imm: 5 },
        Instruction::Blt { rs1: Register::A1, rs2: Register::A2, imm: 8 },
        Instruction::Addi { rd: Register::A0, rs1: Register::ZERO, imm: 0 },
        Instruction::Addi { rd: Register::A0, rs1: Register::ZERO, imm: 1 },
        Instruction::Ecall,
    ];
    assert_eq!(run_program(instructions), 1);
}

#[test]
fn test_bge_taken() {
    let instructions = vec![
        Instruction::Addi { rd: Register::A1, rs1: Register::ZERO, imm: 10 },
        Instruction::Addi { rd: Register::A2, rs1: Register::ZERO, imm: 5 },
        Instruction::Bge { rs1: Register::A1, rs2: Register::A2, imm: 8 },
        Instruction::Addi { rd: Register::A0, rs1: Register::ZERO, imm: 0 },
        Instruction::Addi { rd: Register::A0, rs1: Register::ZERO, imm: 1 },
        Instruction::Ecall,
    ];
    assert_eq!(run_program(instructions), 1);
}

#[test]
fn test_bge_equal() {
    let instructions = vec![
        Instruction::Addi { rd: Register::A1, rs1: Register::ZERO, imm: 5 },
        Instruction::Addi { rd: Register::A2, rs1: Register::ZERO, imm: 5 },
        Instruction::Bge { rs1: Register::A1, rs2: Register::A2, imm: 8 },
        Instruction::Addi { rd: Register::A0, rs1: Register::ZERO, imm: 0 },
        Instruction::Addi { rd: Register::A0, rs1: Register::ZERO, imm: 1 },
        Instruction::Ecall,
    ];
    assert_eq!(run_program(instructions), 1);
}

#[test]
fn test_bltu_unsigned() {
    let instructions = vec![
        Instruction::Addi { rd: Register::A1, rs1: Register::ZERO, imm: 1 },
        Instruction::Addi { rd: Register::A2, rs1: Register::ZERO, imm: -1 }, // Large unsigned
        Instruction::Bltu { rs1: Register::A1, rs2: Register::A2, imm: 8 },
        Instruction::Addi { rd: Register::A0, rs1: Register::ZERO, imm: 0 },
        Instruction::Addi { rd: Register::A0, rs1: Register::ZERO, imm: 1 },
        Instruction::Ecall,
    ];
    assert_eq!(run_program(instructions), 1);
}

#[test]
fn test_bgeu_unsigned() {
    let instructions = vec![
        Instruction::Addi { rd: Register::A1, rs1: Register::ZERO, imm: -1 }, // Large unsigned
        Instruction::Addi { rd: Register::A2, rs1: Register::ZERO, imm: 1 },
        Instruction::Bgeu { rs1: Register::A1, rs2: Register::A2, imm: 8 },
        Instruction::Addi { rd: Register::A0, rs1: Register::ZERO, imm: 0 },
        Instruction::Addi { rd: Register::A0, rs1: Register::ZERO, imm: 1 },
        Instruction::Ecall,
    ];
    assert_eq!(run_program(instructions), 1);
}

// ============================================================================
// JAL tests
// ============================================================================

#[test]
fn test_jal_forward() {
    let instructions = vec![
        Instruction::Addi { rd: Register::A0, rs1: Register::ZERO, imm: 0 },
        Instruction::Jal { rd: Register::RA, imm: 8 }, // Skip next instruction
        Instruction::Addi { rd: Register::A0, rs1: Register::ZERO, imm: 99 }, // Skipped
        Instruction::Addi { rd: Register::A0, rs1: Register::ZERO, imm: 42 },
        Instruction::Ecall,
    ];
    assert_eq!(run_program(instructions), 42);
}

#[test]
fn test_jal_saves_return_address() {
    let instructions = vec![
        Instruction::Jal { rd: Register::A0, imm: 4 }, // Jump to next, save PC+4 to a0
        Instruction::Ecall,
    ];
    // Return address should be PC+4 = 4
    assert_eq!(run_program(instructions), 4);
}

// ============================================================================
// Load/Store tests (LB, LH, LW, LBU, LHU, SB, SH, SW)
// ============================================================================

#[test]
fn test_sw_lw() {
    let instructions = vec![
        Instruction::Addi { rd: Register::A1, rs1: Register::ZERO, imm: 0x100 }, // Address
        Instruction::Addi { rd: Register::A2, rs1: Register::ZERO, imm: 42 },    // Value
        Instruction::Sw { rs1: Register::A1, rs2: Register::A2, imm: 0 },
        Instruction::Lw { rd: Register::A0, rs1: Register::A1, imm: 0 },
        Instruction::Ecall,
    ];
    assert_eq!(run_program(instructions), 42);
}

#[test]
fn test_sw_lw_offset() {
    let instructions = vec![
        Instruction::Addi { rd: Register::A1, rs1: Register::ZERO, imm: 0x100 },
        Instruction::Addi { rd: Register::A2, rs1: Register::ZERO, imm: 123 },
        Instruction::Sw { rs1: Register::A1, rs2: Register::A2, imm: 8 },
        Instruction::Lw { rd: Register::A0, rs1: Register::A1, imm: 8 },
        Instruction::Ecall,
    ];
    assert_eq!(run_program(instructions), 123);
}

#[test]
fn test_sb_lb() {
    let instructions = vec![
        Instruction::Addi { rd: Register::A1, rs1: Register::ZERO, imm: 0x100 },
        Instruction::Addi { rd: Register::A2, rs1: Register::ZERO, imm: 0x7F }, // Positive byte
        Instruction::Sb { rs1: Register::A1, rs2: Register::A2, imm: 0 },
        Instruction::Lb { rd: Register::A0, rs1: Register::A1, imm: 0 },
        Instruction::Ecall,
    ];
    assert_eq!(run_program(instructions), 0x7F);
}

#[test]
fn test_lb_sign_extend() {
    let instructions = vec![
        Instruction::Addi { rd: Register::A1, rs1: Register::ZERO, imm: 0x100 },
        Instruction::Addi { rd: Register::A2, rs1: Register::ZERO, imm: -1 }, // 0xFF as byte
        Instruction::Sb { rs1: Register::A1, rs2: Register::A2, imm: 0 },
        Instruction::Lb { rd: Register::A0, rs1: Register::A1, imm: 0 },
        Instruction::Ecall,
    ];
    // LB should sign-extend 0xFF to -1
    assert_eq!(run_program(instructions) as i32, -1);
}

#[test]
fn test_lbu_zero_extend() {
    let instructions = vec![
        Instruction::Addi { rd: Register::A1, rs1: Register::ZERO, imm: 0x100 },
        Instruction::Addi { rd: Register::A2, rs1: Register::ZERO, imm: -1 },
        Instruction::Sb { rs1: Register::A1, rs2: Register::A2, imm: 0 },
        Instruction::Lbu { rd: Register::A0, rs1: Register::A1, imm: 0 },
        Instruction::Ecall,
    ];
    // LBU should zero-extend 0xFF to 255
    assert_eq!(run_program(instructions), 0xFF);
}

#[test]
fn test_sh_lh() {
    let instructions = vec![
        Instruction::Addi { rd: Register::A1, rs1: Register::ZERO, imm: 0x100 },
        Instruction::Addi { rd: Register::A2, rs1: Register::ZERO, imm: 0x7FFF }, // Max positive short
        Instruction::Sh { rs1: Register::A1, rs2: Register::A2, imm: 0 },
        Instruction::Lh { rd: Register::A0, rs1: Register::A1, imm: 0 },
        Instruction::Ecall,
    ];
    assert_eq!(run_program(instructions), 0x7FFF);
}

#[test]
fn test_lh_sign_extend() {
    let instructions = vec![
        Instruction::Addi { rd: Register::A1, rs1: Register::ZERO, imm: 0x100 },
        Instruction::Addi { rd: Register::A2, rs1: Register::ZERO, imm: -1 },
        Instruction::Sh { rs1: Register::A1, rs2: Register::A2, imm: 0 },
        Instruction::Lh { rd: Register::A0, rs1: Register::A1, imm: 0 },
        Instruction::Ecall,
    ];
    assert_eq!(run_program(instructions) as i32, -1);
}

#[test]
fn test_lhu_zero_extend() {
    let instructions = vec![
        Instruction::Addi { rd: Register::A1, rs1: Register::ZERO, imm: 0x100 },
        Instruction::Addi { rd: Register::A2, rs1: Register::ZERO, imm: -1 },
        Instruction::Sh { rs1: Register::A1, rs2: Register::A2, imm: 0 },
        Instruction::Lhu { rd: Register::A0, rs1: Register::A1, imm: 0 },
        Instruction::Ecall,
    ];
    assert_eq!(run_program(instructions), 0xFFFF);
}

// ============================================================================
// x0 register tests
// ============================================================================

#[test]
fn test_x0_always_zero() {
    let instructions = vec![
        Instruction::Addi { rd: Register::ZERO, rs1: Register::ZERO, imm: 42 }, // Should be ignored
        Instruction::Add { rd: Register::A0, rs1: Register::ZERO, rs2: Register::ZERO },
        Instruction::Ecall,
    ];
    assert_eq!(run_program(instructions), 0);
}
