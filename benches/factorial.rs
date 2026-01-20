//! Factorial benchmark
//!
//! This benchmark compiles a factorial function from RISC-V to SVM
//! and measures SVM compute costs.
//!
//! The factorial function is compiled from C:
//! ```c
//! int factorial(int n) {
//!     int result = 1;
//!     while (n > 1) {
//!         result *= n;
//!         n--;
//!     }
//!     return result;
//! }
//! ```

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use rv32im_svm::{compile, Instruction, Register, SvmVm};

/// Generate RISC-V instructions for factorial(n)
/// The input n should be in a0, result will be in a0
fn factorial_instructions() -> Vec<Instruction> {
    // This implements:
    // int factorial(int n) {
    //     int result = 1;
    //     while (n > 1) {
    //         result *= n;
    //         n--;
    //     }
    //     return result;
    // }
    //
    // RISC-V assembly:
    // factorial:
    //     li      a1, 1           # result = 1
    //     li      a2, 1           # constant 1 for comparison
    // .loop:
    //     bge     a2, a0, .done   # if 1 >= n, exit
    //     mul     a1, a1, a0      # result *= n
    //     addi    a0, a0, -1      # n--
    //     j       .loop
    // .done:
    //     mv      a0, a1          # return result
    //     ecall
    vec![
        // Input: a0 = n
        // a1 = result = 1
        Instruction::Addi { rd: Register::A1, rs1: Register::ZERO, imm: 1 },
        // a2 = 1 (constant for comparison)
        Instruction::Addi { rd: Register::A2, rs1: Register::ZERO, imm: 1 },
        // loop:
        // if n <= 1, exit (using bge a2, a0 which is 1 >= n)
        Instruction::Bge { rs1: Register::A2, rs2: Register::A0, imm: 16 }, // to exit (4 instructions * 4 bytes = 16)
        // result *= n
        Instruction::Mul { rd: Register::A1, rs1: Register::A1, rs2: Register::A0 },
        // n--
        Instruction::Addi { rd: Register::A0, rs1: Register::A0, imm: -1 },
        // jump to loop (back 3 instructions = -12 bytes)
        Instruction::Jal { rd: Register::ZERO, imm: -12 },
        // exit: move result to a0
        Instruction::Add { rd: Register::A0, rs1: Register::A1, rs2: Register::ZERO },
        Instruction::Ecall,
    ]
}

/// Factorial results
#[derive(Debug)]
struct FactorialResult {
    result: u64,
    compute_units: u64,
    instruction_count: usize,
}

/// Run factorial with a specific input value
fn run_factorial(n: i32) -> FactorialResult {
    let instructions = factorial_instructions();
    let program = compile(&instructions).expect("Compilation failed");
    let instruction_count = program.len();

    let mut vm = SvmVm::new(0x20000);
    vm.set_compute_limit(1_000_000);

    // Set input n in register file (a0 = x10)
    // The compiler stores registers at regfile_base (0x8000)
    // a0 is register 10, so address is 0x8000 + 10*4 = 0x8028
    let a0_addr = 0x8000 + 10 * 4;
    let n_bytes = (n as u32).to_le_bytes();
    vm.memory[a0_addr..a0_addr + 4].copy_from_slice(&n_bytes);

    let result = vm.execute(&program).expect("Execution failed");
    let compute_units = vm.compute_units;

    FactorialResult {
        result,
        compute_units,
        instruction_count,
    }
}

fn benchmark_factorial(c: &mut Criterion) {
    let mut group = c.benchmark_group("factorial");

    // Print compilation info once
    let instructions = factorial_instructions();
    let program = compile(&instructions).expect("Compilation failed");
    println!("\n=== Factorial Compilation Info ===");
    println!("RISC-V instructions: {}", instructions.len());
    println!("SVM instructions: {}", program.len());
    println!("Expansion ratio: {:.2}x", program.len() as f64 / instructions.len() as f64);
    println!();

    for n in [3, 5, 7] {
        group.bench_with_input(BenchmarkId::from_parameter(n), &n, |b, &n| {
            b.iter(|| run_factorial(n))
        });
    }

    group.finish();
}

/// Print detailed compute cost analysis
fn print_compute_costs() {
    println!("\n=== SVM Compute Cost Analysis ===\n");
    println!("| n | factorial(n) | Compute Units | SVM Instructions |");
    println!("|---|--------------|---------------|------------------|");

    for n in [3, 5, 7] {
        let result = run_factorial(n);
        println!(
            "| {} | {:>12} | {:>13} | {:>16} |",
            n, result.result, result.compute_units, result.instruction_count
        );
    }
    println!();
}

criterion_group!(benches, benchmark_factorial);
criterion_main!(benches);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_factorial_results() {
        assert_eq!(run_factorial(0).result, 1);
        assert_eq!(run_factorial(1).result, 1);
        assert_eq!(run_factorial(3).result, 6);
        assert_eq!(run_factorial(5).result, 120);
        assert_eq!(run_factorial(7).result, 5040);
        assert_eq!(run_factorial(10).result, 3628800);
    }

    #[test]
    fn print_results() {
        print_compute_costs();
    }
}
