//! Factorial benchmark example
//!
//! This example demonstrates compiling a factorial function from RISC-V
//! to SVM and measuring compute costs.

use rv32im_svm::{compile, Instruction, Register, SvmVm};

/// Generate RISC-V instructions for factorial(n)
fn factorial_instructions() -> Vec<Instruction> {
    vec![
        // a1 = result = 1
        Instruction::Addi { rd: Register::A1, rs1: Register::ZERO, imm: 1 },
        // a2 = 1 (constant for comparison)
        Instruction::Addi { rd: Register::A2, rs1: Register::ZERO, imm: 1 },
        // loop: if n <= 1, exit (using bge a2, a0 which is 1 >= n)
        Instruction::Bge { rs1: Register::A2, rs2: Register::A0, imm: 16 },
        // result *= n
        Instruction::Mul { rd: Register::A1, rs1: Register::A1, rs2: Register::A0 },
        // n--
        Instruction::Addi { rd: Register::A0, rs1: Register::A0, imm: -1 },
        // jump to loop
        Instruction::Jal { rd: Register::ZERO, imm: -12 },
        // exit: move result to a0
        Instruction::Add { rd: Register::A0, rs1: Register::A1, rs2: Register::ZERO },
        Instruction::Ecall,
    ]
}

/// Run factorial and return (result, compute_units)
fn run_factorial(n: i32) -> (u64, u64) {
    let instructions = factorial_instructions();
    let program = compile(&instructions).expect("Compilation failed");

    let mut vm = SvmVm::new(0x20000);
    vm.set_compute_limit(1_000_000);

    // Set input n in register file (a0 = x10)
    let a0_addr = 0x8000 + 10 * 4;
    let n_bytes = (n as u32).to_le_bytes();
    vm.memory[a0_addr..a0_addr + 4].copy_from_slice(&n_bytes);

    let result = vm.execute(&program).expect("Execution failed");
    (result, vm.compute_units)
}

fn main() {
    let instructions = factorial_instructions();
    let program = compile(&instructions).expect("Compilation failed");

    println!("=== rv32im to SVM Factorial Benchmark ===\n");
    println!("Compilation Statistics:");
    println!("  RISC-V instructions: {}", instructions.len());
    println!("  SVM instructions: {}", program.len());
    println!("  Expansion ratio: {:.2}x", program.len() as f64 / instructions.len() as f64);
    println!("  Estimated base compute: {} CUs", program.compute_cost());
    println!();

    println!("Execution Results:");
    println!("| n  | factorial(n) | Compute Units |");
    println!("|----|--------------|---------------|");

    for n in [3, 5, 7, 10, 12] {
        let (result, cu) = run_factorial(n);
        println!("| {:>2} | {:>12} | {:>13} |", n, result, cu);
    }

    println!();
    println!("Note: Compute units increase with loop iterations.");
    println!("Each iteration involves:");
    println!("  - Loading/storing RISC-V registers from memory");
    println!("  - Branch condition evaluation");
    println!("  - Multiplication operation (10 CUs)");
    println!("  - Arithmetic operations (1 CU each)");
}
