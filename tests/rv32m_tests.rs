//! RISC-V rv32m (multiply/divide) extension tests
//!
//! These tests cover the M extension instructions.

use rv32im_svm::{compile, execute, Instruction, Register};

fn run_program(instructions: Vec<Instruction>) -> u64 {
    let program = compile(&instructions).expect("Compilation failed");
    execute(&program).expect("Execution failed")
}

// Helper to load a 32-bit value into a register
fn load_value(reg: Register, val: i32) -> Vec<Instruction> {
    if val >= -2048 && val <= 2047 {
        vec![Instruction::Addi { rd: reg, rs1: Register::ZERO, imm: val }]
    } else {
        // Use LUI + ADDI
        let upper = (val as u32 + 0x800) & 0xFFFFF000;
        let lower = val - upper as i32;
        vec![
            Instruction::Lui { rd: reg, imm: upper as i32 },
            Instruction::Addi { rd: reg, rs1: reg, imm: lower },
        ]
    }
}

// ============================================================================
// MUL tests
// ============================================================================

#[test]
fn test_mul_positive() {
    let mut instructions = vec![];
    instructions.extend(load_value(Register::A1, 6));
    instructions.extend(load_value(Register::A2, 7));
    instructions.push(Instruction::Mul { rd: Register::A0, rs1: Register::A1, rs2: Register::A2 });
    instructions.push(Instruction::Ecall);
    assert_eq!(run_program(instructions), 42);
}

#[test]
fn test_mul_zero() {
    let mut instructions = vec![];
    instructions.extend(load_value(Register::A1, 100));
    instructions.extend(load_value(Register::A2, 0));
    instructions.push(Instruction::Mul { rd: Register::A0, rs1: Register::A1, rs2: Register::A2 });
    instructions.push(Instruction::Ecall);
    assert_eq!(run_program(instructions), 0);
}

#[test]
fn test_mul_negative() {
    let mut instructions = vec![];
    instructions.extend(load_value(Register::A1, -6));
    instructions.extend(load_value(Register::A2, 7));
    instructions.push(Instruction::Mul { rd: Register::A0, rs1: Register::A1, rs2: Register::A2 });
    instructions.push(Instruction::Ecall);
    assert_eq!(run_program(instructions) as i32, -42);
}

#[test]
fn test_mul_both_negative() {
    let mut instructions = vec![];
    instructions.extend(load_value(Register::A1, -6));
    instructions.extend(load_value(Register::A2, -7));
    instructions.push(Instruction::Mul { rd: Register::A0, rs1: Register::A1, rs2: Register::A2 });
    instructions.push(Instruction::Ecall);
    assert_eq!(run_program(instructions), 42);
}

#[test]
fn test_mul_large() {
    let mut instructions = vec![];
    instructions.extend(load_value(Register::A1, 1000));
    instructions.extend(load_value(Register::A2, 1000));
    instructions.push(Instruction::Mul { rd: Register::A0, rs1: Register::A1, rs2: Register::A2 });
    instructions.push(Instruction::Ecall);
    assert_eq!(run_program(instructions), 1_000_000);
}

#[test]
fn test_mul_overflow() {
    // Result wraps around (only lower 32 bits)
    let mut instructions = vec![];
    instructions.extend(load_value(Register::A1, 0x10000));
    instructions.extend(load_value(Register::A2, 0x10000));
    instructions.push(Instruction::Mul { rd: Register::A0, rs1: Register::A1, rs2: Register::A2 });
    instructions.push(Instruction::Ecall);
    // 0x10000 * 0x10000 = 0x100000000, which wraps to 0
    assert_eq!(run_program(instructions) as u32, 0);
}

// ============================================================================
// MULH tests (signed x signed -> upper 32 bits)
// ============================================================================

#[test]
fn test_mulh_small() {
    // Small numbers should have 0 in upper bits
    let mut instructions = vec![];
    instructions.extend(load_value(Register::A1, 100));
    instructions.extend(load_value(Register::A2, 100));
    instructions.push(Instruction::Mulh { rd: Register::A0, rs1: Register::A1, rs2: Register::A2 });
    instructions.push(Instruction::Ecall);
    assert_eq!(run_program(instructions), 0);
}

#[test]
fn test_mulh_large() {
    let mut instructions = vec![];
    instructions.extend(load_value(Register::A1, 0x40000000)); // 2^30
    instructions.extend(load_value(Register::A2, 4));
    instructions.push(Instruction::Mulh { rd: Register::A0, rs1: Register::A1, rs2: Register::A2 });
    instructions.push(Instruction::Ecall);
    // 2^30 * 4 = 2^32, upper 32 bits = 1
    assert_eq!(run_program(instructions), 1);
}

#[test]
fn test_mulh_negative() {
    let mut instructions = vec![];
    instructions.extend(load_value(Register::A1, -1));
    instructions.extend(load_value(Register::A2, -1));
    instructions.push(Instruction::Mulh { rd: Register::A0, rs1: Register::A1, rs2: Register::A2 });
    instructions.push(Instruction::Ecall);
    // (-1) * (-1) = 1, upper 32 bits = 0
    assert_eq!(run_program(instructions), 0);
}

// ============================================================================
// MULHU tests (unsigned x unsigned -> upper 32 bits)
// ============================================================================

#[test]
fn test_mulhu_large() {
    let mut instructions = vec![];
    instructions.extend(load_value(Register::A1, -1)); // 0xFFFFFFFF unsigned
    instructions.extend(load_value(Register::A2, 2));
    instructions.push(Instruction::Mulhu { rd: Register::A0, rs1: Register::A1, rs2: Register::A2 });
    instructions.push(Instruction::Ecall);
    // 0xFFFFFFFF * 2 = 0x1FFFFFFFE, upper = 1
    assert_eq!(run_program(instructions), 1);
}

// ============================================================================
// MULHSU tests (signed x unsigned -> upper 32 bits)
// ============================================================================

#[test]
fn test_mulhsu_positive() {
    let mut instructions = vec![];
    instructions.extend(load_value(Register::A1, 0x40000000));
    instructions.extend(load_value(Register::A2, 4));
    instructions.push(Instruction::Mulhsu { rd: Register::A0, rs1: Register::A1, rs2: Register::A2 });
    instructions.push(Instruction::Ecall);
    assert_eq!(run_program(instructions), 1);
}

// ============================================================================
// DIV tests (signed division)
// ============================================================================

#[test]
fn test_div_positive() {
    let mut instructions = vec![];
    instructions.extend(load_value(Register::A1, 42));
    instructions.extend(load_value(Register::A2, 6));
    instructions.push(Instruction::Div { rd: Register::A0, rs1: Register::A1, rs2: Register::A2 });
    instructions.push(Instruction::Ecall);
    assert_eq!(run_program(instructions), 7);
}

#[test]
fn test_div_negative_dividend() {
    let mut instructions = vec![];
    instructions.extend(load_value(Register::A1, -42));
    instructions.extend(load_value(Register::A2, 6));
    instructions.push(Instruction::Div { rd: Register::A0, rs1: Register::A1, rs2: Register::A2 });
    instructions.push(Instruction::Ecall);
    assert_eq!(run_program(instructions) as i32, -7);
}

#[test]
fn test_div_negative_divisor() {
    let mut instructions = vec![];
    instructions.extend(load_value(Register::A1, 42));
    instructions.extend(load_value(Register::A2, -6));
    instructions.push(Instruction::Div { rd: Register::A0, rs1: Register::A1, rs2: Register::A2 });
    instructions.push(Instruction::Ecall);
    assert_eq!(run_program(instructions) as i32, -7);
}

#[test]
fn test_div_both_negative() {
    let mut instructions = vec![];
    instructions.extend(load_value(Register::A1, -42));
    instructions.extend(load_value(Register::A2, -6));
    instructions.push(Instruction::Div { rd: Register::A0, rs1: Register::A1, rs2: Register::A2 });
    instructions.push(Instruction::Ecall);
    assert_eq!(run_program(instructions), 7);
}

#[test]
fn test_div_by_zero() {
    // Division by zero should return -1
    let mut instructions = vec![];
    instructions.extend(load_value(Register::A1, 42));
    instructions.extend(load_value(Register::A2, 0));
    instructions.push(Instruction::Div { rd: Register::A0, rs1: Register::A1, rs2: Register::A2 });
    instructions.push(Instruction::Ecall);
    assert_eq!(run_program(instructions) as i32, -1);
}

#[test]
fn test_div_overflow() {
    // INT_MIN / -1 should return INT_MIN (overflow case)
    let mut instructions = vec![];
    instructions.extend(load_value(Register::A1, i32::MIN));
    instructions.extend(load_value(Register::A2, -1));
    instructions.push(Instruction::Div { rd: Register::A0, rs1: Register::A1, rs2: Register::A2 });
    instructions.push(Instruction::Ecall);
    assert_eq!(run_program(instructions) as i32, i32::MIN);
}

#[test]
fn test_div_truncate() {
    // Division truncates toward zero
    let mut instructions = vec![];
    instructions.extend(load_value(Register::A1, 7));
    instructions.extend(load_value(Register::A2, 3));
    instructions.push(Instruction::Div { rd: Register::A0, rs1: Register::A1, rs2: Register::A2 });
    instructions.push(Instruction::Ecall);
    assert_eq!(run_program(instructions), 2);
}

// ============================================================================
// DIVU tests (unsigned division)
// ============================================================================

#[test]
fn test_divu_positive() {
    let mut instructions = vec![];
    instructions.extend(load_value(Register::A1, 42));
    instructions.extend(load_value(Register::A2, 6));
    instructions.push(Instruction::Divu { rd: Register::A0, rs1: Register::A1, rs2: Register::A2 });
    instructions.push(Instruction::Ecall);
    assert_eq!(run_program(instructions), 7);
}

#[test]
fn test_divu_large() {
    // -1 as unsigned is 0xFFFFFFFF
    let mut instructions = vec![];
    instructions.extend(load_value(Register::A1, -1));
    instructions.extend(load_value(Register::A2, 2));
    instructions.push(Instruction::Divu { rd: Register::A0, rs1: Register::A1, rs2: Register::A2 });
    instructions.push(Instruction::Ecall);
    // 0xFFFFFFFF / 2 = 0x7FFFFFFF
    assert_eq!(run_program(instructions) as u32, 0x7FFFFFFF);
}

#[test]
fn test_divu_by_zero() {
    // Unsigned division by zero returns max value
    let mut instructions = vec![];
    instructions.extend(load_value(Register::A1, 42));
    instructions.extend(load_value(Register::A2, 0));
    instructions.push(Instruction::Divu { rd: Register::A0, rs1: Register::A1, rs2: Register::A2 });
    instructions.push(Instruction::Ecall);
    assert_eq!(run_program(instructions) as u32, 0xFFFFFFFF);
}

// ============================================================================
// REM tests (signed remainder)
// ============================================================================

#[test]
fn test_rem_positive() {
    let mut instructions = vec![];
    instructions.extend(load_value(Register::A1, 7));
    instructions.extend(load_value(Register::A2, 3));
    instructions.push(Instruction::Rem { rd: Register::A0, rs1: Register::A1, rs2: Register::A2 });
    instructions.push(Instruction::Ecall);
    assert_eq!(run_program(instructions), 1);
}

#[test]
fn test_rem_negative_dividend() {
    // Remainder has same sign as dividend
    let mut instructions = vec![];
    instructions.extend(load_value(Register::A1, -7));
    instructions.extend(load_value(Register::A2, 3));
    instructions.push(Instruction::Rem { rd: Register::A0, rs1: Register::A1, rs2: Register::A2 });
    instructions.push(Instruction::Ecall);
    assert_eq!(run_program(instructions) as i32, -1);
}

#[test]
fn test_rem_negative_divisor() {
    let mut instructions = vec![];
    instructions.extend(load_value(Register::A1, 7));
    instructions.extend(load_value(Register::A2, -3));
    instructions.push(Instruction::Rem { rd: Register::A0, rs1: Register::A1, rs2: Register::A2 });
    instructions.push(Instruction::Ecall);
    assert_eq!(run_program(instructions), 1);
}

#[test]
fn test_rem_both_negative() {
    let mut instructions = vec![];
    instructions.extend(load_value(Register::A1, -7));
    instructions.extend(load_value(Register::A2, -3));
    instructions.push(Instruction::Rem { rd: Register::A0, rs1: Register::A1, rs2: Register::A2 });
    instructions.push(Instruction::Ecall);
    assert_eq!(run_program(instructions) as i32, -1);
}

#[test]
fn test_rem_by_zero() {
    // Remainder by zero returns dividend
    let mut instructions = vec![];
    instructions.extend(load_value(Register::A1, 42));
    instructions.extend(load_value(Register::A2, 0));
    instructions.push(Instruction::Rem { rd: Register::A0, rs1: Register::A1, rs2: Register::A2 });
    instructions.push(Instruction::Ecall);
    assert_eq!(run_program(instructions), 42);
}

#[test]
fn test_rem_overflow() {
    // INT_MIN % -1 should return 0 (overflow case)
    let mut instructions = vec![];
    instructions.extend(load_value(Register::A1, i32::MIN));
    instructions.extend(load_value(Register::A2, -1));
    instructions.push(Instruction::Rem { rd: Register::A0, rs1: Register::A1, rs2: Register::A2 });
    instructions.push(Instruction::Ecall);
    assert_eq!(run_program(instructions), 0);
}

// ============================================================================
// REMU tests (unsigned remainder)
// ============================================================================

#[test]
fn test_remu_positive() {
    let mut instructions = vec![];
    instructions.extend(load_value(Register::A1, 7));
    instructions.extend(load_value(Register::A2, 3));
    instructions.push(Instruction::Remu { rd: Register::A0, rs1: Register::A1, rs2: Register::A2 });
    instructions.push(Instruction::Ecall);
    assert_eq!(run_program(instructions), 1);
}

#[test]
fn test_remu_large() {
    let mut instructions = vec![];
    instructions.extend(load_value(Register::A1, -1)); // 0xFFFFFFFF
    instructions.extend(load_value(Register::A2, 7));
    instructions.push(Instruction::Remu { rd: Register::A0, rs1: Register::A1, rs2: Register::A2 });
    instructions.push(Instruction::Ecall);
    // 0xFFFFFFFF % 7 = 3
    assert_eq!(run_program(instructions), 3);
}

#[test]
fn test_remu_by_zero() {
    let mut instructions = vec![];
    instructions.extend(load_value(Register::A1, 42));
    instructions.extend(load_value(Register::A2, 0));
    instructions.push(Instruction::Remu { rd: Register::A0, rs1: Register::A1, rs2: Register::A2 });
    instructions.push(Instruction::Ecall);
    assert_eq!(run_program(instructions), 42);
}

// ============================================================================
// Combined tests
// ============================================================================

// BLE is actually BGE with swapped operands, so use BGE directly
#[test]
fn test_factorial_5_v2() {
    // Compute factorial(5) = 120
    let instructions = vec![
        // n = 5
        Instruction::Addi { rd: Register::A1, rs1: Register::ZERO, imm: 5 },
        // result = 1
        Instruction::Addi { rd: Register::A0, rs1: Register::ZERO, imm: 1 },
        // loop:
        // if n < 2, exit (blt n, 2 => exit)
        Instruction::Addi { rd: Register::A2, rs1: Register::ZERO, imm: 2 },
        Instruction::Blt { rs1: Register::A1, rs2: Register::A2, imm: 16 },
        // result *= n
        Instruction::Mul { rd: Register::A0, rs1: Register::A0, rs2: Register::A1 },
        // n--
        Instruction::Addi { rd: Register::A1, rs1: Register::A1, imm: -1 },
        // jump to loop
        Instruction::Jal { rd: Register::ZERO, imm: -16 },
        // exit:
        Instruction::Ecall,
    ];
    assert_eq!(run_program(instructions), 120);
}

#[test]
fn test_gcd() {
    // Compute GCD(48, 18) = 6 using Euclidean algorithm
    let instructions = vec![
        // a = 48, b = 18
        Instruction::Addi { rd: Register::A1, rs1: Register::ZERO, imm: 48 },
        Instruction::Addi { rd: Register::A2, rs1: Register::ZERO, imm: 18 },
        // while b != 0:
        Instruction::Beq { rs1: Register::A2, rs2: Register::ZERO, imm: 20 }, // exit
        // t = b
        Instruction::Add { rd: Register::A3, rs1: Register::A2, rs2: Register::ZERO },
        // b = a % b
        Instruction::Rem { rd: Register::A2, rs1: Register::A1, rs2: Register::A2 },
        // a = t
        Instruction::Add { rd: Register::A1, rs1: Register::A3, rs2: Register::ZERO },
        // loop
        Instruction::Jal { rd: Register::ZERO, imm: -16 },
        // exit: return a
        Instruction::Add { rd: Register::A0, rs1: Register::A1, rs2: Register::ZERO },
        Instruction::Ecall,
    ];
    assert_eq!(run_program(instructions), 6);
}

#[test]
fn test_sum_1_to_10() {
    // Compute sum of 1 to 10 = 55
    let instructions = vec![
        // sum = 0
        Instruction::Addi { rd: Register::A0, rs1: Register::ZERO, imm: 0 },
        // i = 1
        Instruction::Addi { rd: Register::A1, rs1: Register::ZERO, imm: 1 },
        // loop:
        // if i > 10, exit
        Instruction::Addi { rd: Register::A2, rs1: Register::ZERO, imm: 10 },
        Instruction::Blt { rs1: Register::A2, rs2: Register::A1, imm: 16 }, // i > 10 means 10 < i
        // sum += i
        Instruction::Add { rd: Register::A0, rs1: Register::A0, rs2: Register::A1 },
        // i++
        Instruction::Addi { rd: Register::A1, rs1: Register::A1, imm: 1 },
        // loop
        Instruction::Jal { rd: Register::ZERO, imm: -16 },
        // exit:
        Instruction::Ecall,
    ];
    assert_eq!(run_program(instructions), 55);
}
