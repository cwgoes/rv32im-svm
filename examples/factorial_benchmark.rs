//! Mathematical functions benchmark
//!
//! This example demonstrates compiling various mathematical functions
//! from RISC-V to SVM and measuring compute costs.

use rv32im_svm::{compile, Instruction, Register, SvmVm};

/// Benchmark result for a single function execution
#[derive(Debug, Clone)]
struct BenchmarkResult {
    name: &'static str,
    input: String,
    result: u64,
    rv_instructions: usize,
    svm_instructions: usize,
    compute_units: u64,
}

impl BenchmarkResult {
    fn expansion_ratio(&self) -> f64 {
        self.svm_instructions as f64 / self.rv_instructions as f64
    }

    fn cu_per_rv_instruction(&self) -> f64 {
        self.compute_units as f64 / self.rv_instructions as f64
    }
}

/// Run a program with input in a0 and return (result, compute_units)
fn run_program(instructions: &[Instruction], input: i32) -> (u64, u64) {
    let program = compile(instructions).expect("Compilation failed");

    let mut vm = SvmVm::new(0x20000);
    vm.set_compute_limit(10_000_000);

    // Set input in a0 (register 10)
    let a0_addr = 0x8000 + 10 * 4;
    vm.memory[a0_addr..a0_addr + 4].copy_from_slice(&(input as u32).to_le_bytes());

    let result = vm.execute(&program).expect("Execution failed");
    (result, vm.compute_units)
}

/// Run a program with two inputs in a0 and a1
fn run_program_2(instructions: &[Instruction], a: i32, b: i32) -> (u64, u64) {
    let program = compile(instructions).expect("Compilation failed");

    let mut vm = SvmVm::new(0x20000);
    vm.set_compute_limit(10_000_000);

    // Set inputs
    let a0_addr = 0x8000 + 10 * 4;
    let a1_addr = 0x8000 + 11 * 4;
    vm.memory[a0_addr..a0_addr + 4].copy_from_slice(&(a as u32).to_le_bytes());
    vm.memory[a1_addr..a1_addr + 4].copy_from_slice(&(b as u32).to_le_bytes());

    let result = vm.execute(&program).expect("Execution failed");
    (result, vm.compute_units)
}

// ============================================================================
// Mathematical Functions
// ============================================================================

/// Factorial: n! = n * (n-1) * ... * 1
fn factorial_instructions() -> Vec<Instruction> {
    vec![
        // a1 = result = 1
        Instruction::Addi { rd: Register::A1, rs1: Register::ZERO, imm: 1 },
        // a2 = 1 (constant)
        Instruction::Addi { rd: Register::A2, rs1: Register::ZERO, imm: 1 },
        // loop: if n <= 1, exit
        Instruction::Bge { rs1: Register::A2, rs2: Register::A0, imm: 16 },
        // result *= n
        Instruction::Mul { rd: Register::A1, rs1: Register::A1, rs2: Register::A0 },
        // n--
        Instruction::Addi { rd: Register::A0, rs1: Register::A0, imm: -1 },
        // jump to loop
        Instruction::Jal { rd: Register::ZERO, imm: -12 },
        // exit: return result
        Instruction::Add { rd: Register::A0, rs1: Register::A1, rs2: Register::ZERO },
        Instruction::Ecall,
    ]
}

/// Fibonacci: fib(n) = fib(n-1) + fib(n-2) (iterative)
fn fibonacci_instructions() -> Vec<Instruction> {
    vec![
        // if n <= 1, return n
        Instruction::Addi { rd: Register::A2, rs1: Register::ZERO, imm: 1 },
        Instruction::Bge { rs1: Register::A2, rs2: Register::A0, imm: 40 }, // to return n
        // a1 = fib(n-2) = 0
        Instruction::Addi { rd: Register::A1, rs1: Register::ZERO, imm: 0 },
        // a2 = fib(n-1) = 1
        Instruction::Addi { rd: Register::A2, rs1: Register::ZERO, imm: 1 },
        // a3 = counter = n - 1
        Instruction::Addi { rd: Register::A3, rs1: Register::A0, imm: -1 },
        // loop: if counter <= 0, exit
        Instruction::Bge { rs1: Register::ZERO, rs2: Register::A3, imm: 24 },
        // a4 = a1 + a2 (next fib)
        Instruction::Add { rd: Register::A4, rs1: Register::A1, rs2: Register::A2 },
        // a1 = a2
        Instruction::Add { rd: Register::A1, rs1: Register::A2, rs2: Register::ZERO },
        // a2 = a4
        Instruction::Add { rd: Register::A2, rs1: Register::A4, rs2: Register::ZERO },
        // counter--
        Instruction::Addi { rd: Register::A3, rs1: Register::A3, imm: -1 },
        // jump to loop
        Instruction::Jal { rd: Register::ZERO, imm: -20 },
        // return a2
        Instruction::Add { rd: Register::A0, rs1: Register::A2, rs2: Register::ZERO },
        Instruction::Ecall,
    ]
}

/// GCD using Euclidean algorithm: gcd(a, b)
fn gcd_instructions() -> Vec<Instruction> {
    vec![
        // while b != 0:
        Instruction::Beq { rs1: Register::A1, rs2: Register::ZERO, imm: 20 }, // exit
        // t = b
        Instruction::Add { rd: Register::A2, rs1: Register::A1, rs2: Register::ZERO },
        // b = a % b
        Instruction::Rem { rd: Register::A1, rs1: Register::A0, rs2: Register::A1 },
        // a = t
        Instruction::Add { rd: Register::A0, rs1: Register::A2, rs2: Register::ZERO },
        // loop
        Instruction::Jal { rd: Register::ZERO, imm: -16 },
        // exit: return a
        Instruction::Ecall,
    ]
}

/// Power: a^b (integer exponentiation)
/// Layout: PC=0 init, PC=4 branch, PC=8 mul, PC=12 dec, PC=16 loop, PC=20 return, PC=24 ecall
fn power_instructions() -> Vec<Instruction> {
    vec![
        // PC=0: result = 1
        Instruction::Addi { rd: Register::A2, rs1: Register::ZERO, imm: 1 },
        // PC=4: while b > 0 (if 0 >= b, exit to PC=20)
        Instruction::Bge { rs1: Register::ZERO, rs2: Register::A1, imm: 16 },
        // PC=8: result *= a
        Instruction::Mul { rd: Register::A2, rs1: Register::A2, rs2: Register::A0 },
        // PC=12: b--
        Instruction::Addi { rd: Register::A1, rs1: Register::A1, imm: -1 },
        // PC=16: loop back to PC=4
        Instruction::Jal { rd: Register::ZERO, imm: -12 },
        // PC=20: return result
        Instruction::Add { rd: Register::A0, rs1: Register::A2, rs2: Register::ZERO },
        // PC=24: ecall
        Instruction::Ecall,
    ]
}

/// Sum of squares: 1^2 + 2^2 + ... + n^2
/// Layout: PC=0,4 init, PC=8 branch, PC=12 mul, PC=16 add, PC=20 inc, PC=24 loop, PC=28 return, PC=32 ecall
fn sum_squares_instructions() -> Vec<Instruction> {
    vec![
        // PC=0: sum = 0
        Instruction::Addi { rd: Register::A1, rs1: Register::ZERO, imm: 0 },
        // PC=4: i = 1
        Instruction::Addi { rd: Register::A2, rs1: Register::ZERO, imm: 1 },
        // PC=8: while i <= n (if n < i, exit to PC=28)
        Instruction::Blt { rs1: Register::A0, rs2: Register::A2, imm: 20 },
        // PC=12: sq = i * i
        Instruction::Mul { rd: Register::A3, rs1: Register::A2, rs2: Register::A2 },
        // PC=16: sum += sq
        Instruction::Add { rd: Register::A1, rs1: Register::A1, rs2: Register::A3 },
        // PC=20: i++
        Instruction::Addi { rd: Register::A2, rs1: Register::A2, imm: 1 },
        // PC=24: loop back to PC=8
        Instruction::Jal { rd: Register::ZERO, imm: -16 },
        // PC=28: return sum
        Instruction::Add { rd: Register::A0, rs1: Register::A1, rs2: Register::ZERO },
        // PC=32: ecall
        Instruction::Ecall,
    ]
}

/// Integer square root (linear search - simpler but correct)
/// Returns floor(sqrt(n))
fn isqrt_instructions() -> Vec<Instruction> {
    // Simple approach: start from 1, increment until i*i > n
    // PC layout: 0,4 init, 8 loop check, 12 mul, 16 compare, 20 inc, 24 loop, 28 return, 32 ecall
    vec![
        // PC=0: if n == 0, return 0 (branch to ecall at PC=32)
        Instruction::Beq { rs1: Register::A0, rs2: Register::ZERO, imm: 32 },
        // PC=4: i = 1
        Instruction::Addi { rd: Register::A1, rs1: Register::ZERO, imm: 1 },
        // PC=8: sq = i * i
        Instruction::Mul { rd: Register::A2, rs1: Register::A1, rs2: Register::A1 },
        // PC=12: if sq > n, return i-1 (branch to PC=24)
        Instruction::Blt { rs1: Register::A0, rs2: Register::A2, imm: 12 },
        // PC=16: i++
        Instruction::Addi { rd: Register::A1, rs1: Register::A1, imm: 1 },
        // PC=20: loop back to PC=8
        Instruction::Jal { rd: Register::ZERO, imm: -12 },
        // PC=24: return i - 1
        Instruction::Addi { rd: Register::A0, rs1: Register::A1, imm: -1 },
        // PC=28: ecall
        Instruction::Ecall,
    ]
}

/// Modular exponentiation: (base^exp) mod m using binary exponentiation
/// PC layout: 0 init, 4 base%m, 8 loop check, 12-16 if odd, 20-24 result update, 28 shift, 32-36 base^2, 40 loop, 44 return, 48 ecall
fn modpow_instructions() -> Vec<Instruction> {
    // a0 = base, a1 = exp, a2 = m
    vec![
        // PC=0: result = 1
        Instruction::Addi { rd: Register::A3, rs1: Register::ZERO, imm: 1 },
        // PC=4: base = base % m
        Instruction::Remu { rd: Register::A0, rs1: Register::A0, rs2: Register::A2 },
        // PC=8: while exp > 0 (if 0 >= exp, exit to PC=44)
        Instruction::Bge { rs1: Register::ZERO, rs2: Register::A1, imm: 36 },
        // PC=12: if exp & 1
        Instruction::Andi { rd: Register::A4, rs1: Register::A1, imm: 1 },
        // PC=16: if (exp & 1) == 0, skip to PC=28
        Instruction::Beq { rs1: Register::A4, rs2: Register::ZERO, imm: 12 },
        // PC=20: result = (result * base) % m
        Instruction::Mul { rd: Register::A3, rs1: Register::A3, rs2: Register::A0 },
        // PC=24:
        Instruction::Remu { rd: Register::A3, rs1: Register::A3, rs2: Register::A2 },
        // PC=28: exp = exp >> 1
        Instruction::Srli { rd: Register::A1, rs1: Register::A1, shamt: 1 },
        // PC=32: base = (base * base) % m
        Instruction::Mul { rd: Register::A0, rs1: Register::A0, rs2: Register::A0 },
        // PC=36:
        Instruction::Remu { rd: Register::A0, rs1: Register::A0, rs2: Register::A2 },
        // PC=40: loop back to PC=8
        Instruction::Jal { rd: Register::ZERO, imm: -32 },
        // PC=44: return result
        Instruction::Add { rd: Register::A0, rs1: Register::A3, rs2: Register::ZERO },
        // PC=48: ecall
        Instruction::Ecall,
    ]
}

/// Run a program with three inputs (for modpow)
fn run_program_3(instructions: &[Instruction], a: i32, b: i32, c: i32) -> (u64, u64) {
    let program = compile(instructions).expect("Compilation failed");

    let mut vm = SvmVm::new(0x20000);
    vm.set_compute_limit(10_000_000);

    let a0_addr = 0x8000 + 10 * 4;
    let a1_addr = 0x8000 + 11 * 4;
    let a2_addr = 0x8000 + 12 * 4;
    vm.memory[a0_addr..a0_addr + 4].copy_from_slice(&(a as u32).to_le_bytes());
    vm.memory[a1_addr..a1_addr + 4].copy_from_slice(&(b as u32).to_le_bytes());
    vm.memory[a2_addr..a2_addr + 4].copy_from_slice(&(c as u32).to_le_bytes());

    let result = vm.execute(&program).expect("Execution failed");
    (result, vm.compute_units)
}

fn main() {
    println!("=== rv32im to SVM Mathematical Functions Benchmark ===\n");

    let mut results: Vec<BenchmarkResult> = Vec::new();

    // Factorial benchmarks
    let factorial = factorial_instructions();
    let factorial_svm = compile(&factorial).unwrap().len();
    for n in [5, 7, 10] {
        let (result, cu) = run_program(&factorial, n);
        results.push(BenchmarkResult {
            name: "factorial",
            input: format!("{}", n),
            result,
            rv_instructions: factorial.len(),
            svm_instructions: factorial_svm,
            compute_units: cu,
        });
    }

    // Fibonacci benchmarks
    let fibonacci = fibonacci_instructions();
    let fibonacci_svm = compile(&fibonacci).unwrap().len();
    for n in [10, 15, 20] {
        let (result, cu) = run_program(&fibonacci, n);
        results.push(BenchmarkResult {
            name: "fibonacci",
            input: format!("{}", n),
            result,
            rv_instructions: fibonacci.len(),
            svm_instructions: fibonacci_svm,
            compute_units: cu,
        });
    }

    // GCD benchmarks
    let gcd = gcd_instructions();
    let gcd_svm = compile(&gcd).unwrap().len();
    for (a, b) in [(48, 18), (1071, 462), (10000, 7777)] {
        let (result, cu) = run_program_2(&gcd, a, b);
        results.push(BenchmarkResult {
            name: "gcd",
            input: format!("{}, {}", a, b),
            result,
            rv_instructions: gcd.len(),
            svm_instructions: gcd_svm,
            compute_units: cu,
        });
    }

    // Power benchmarks
    let power = power_instructions();
    let power_svm = compile(&power).unwrap().len();
    for (a, b) in [(2, 10), (3, 7), (5, 5)] {
        let (result, cu) = run_program_2(&power, a, b);
        results.push(BenchmarkResult {
            name: "power",
            input: format!("{}^{}", a, b),
            result,
            rv_instructions: power.len(),
            svm_instructions: power_svm,
            compute_units: cu,
        });
    }

    // Sum of squares benchmarks
    let sum_sq = sum_squares_instructions();
    let sum_sq_svm = compile(&sum_sq).unwrap().len();
    for n in [10, 50, 100] {
        let (result, cu) = run_program(&sum_sq, n);
        results.push(BenchmarkResult {
            name: "sum_squares",
            input: format!("{}", n),
            result,
            rv_instructions: sum_sq.len(),
            svm_instructions: sum_sq_svm,
            compute_units: cu,
        });
    }

    // Integer square root benchmarks
    let isqrt = isqrt_instructions();
    let isqrt_svm = compile(&isqrt).unwrap().len();
    for n in [100, 10000, 1000000] {
        let (result, cu) = run_program(&isqrt, n);
        results.push(BenchmarkResult {
            name: "isqrt",
            input: format!("{}", n),
            result,
            rv_instructions: isqrt.len(),
            svm_instructions: isqrt_svm,
            compute_units: cu,
        });
    }

    // Modular exponentiation benchmarks
    let modpow = modpow_instructions();
    let modpow_svm = compile(&modpow).unwrap().len();
    for (base, exp, m) in [(2, 10, 1000), (3, 13, 1000007), (7, 20, 1000000007)] {
        let (result, cu) = run_program_3(&modpow, base, exp, m);
        results.push(BenchmarkResult {
            name: "modpow",
            input: format!("{}^{} mod {}", base, exp, m),
            result,
            rv_instructions: modpow.len(),
            svm_instructions: modpow_svm,
            compute_units: cu,
        });
    }

    // Print compilation statistics
    println!("Compilation Statistics:");
    println!("| Function     | RV32IM Insts | SVM Insts | Expansion |");
    println!("|--------------|--------------|-----------|-----------|");
    let unique_funcs: Vec<_> = results.iter()
        .map(|r| (r.name, r.rv_instructions, r.svm_instructions))
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect();
    for (name, rv, svm) in &unique_funcs {
        println!("| {:12} | {:>12} | {:>9} | {:>8.2}x |",
                 name, rv, svm, *svm as f64 / *rv as f64);
    }
    println!();

    // Print execution results
    println!("Execution Results:");
    println!("| Function     | Input              | Result       | Compute Units |");
    println!("|--------------|--------------------|--------------| --------------|");
    for r in &results {
        println!("| {:12} | {:>18} | {:>12} | {:>13} |",
                 r.name, r.input, r.result, r.compute_units);
    }
    println!();

    // Calculate and print overhead ratios
    println!("=== SVM Overhead Analysis ===\n");

    let total_rv_instructions: usize = results.iter().map(|r| r.rv_instructions).sum();
    let total_svm_instructions: usize = results.iter().map(|r| r.svm_instructions).sum();
    let total_compute_units: u64 = results.iter().map(|r| r.compute_units).sum();

    let mean_expansion = total_svm_instructions as f64 / total_rv_instructions as f64;
    let mean_cu_per_rv = total_compute_units as f64 / results.len() as f64
        / (total_rv_instructions as f64 / results.len() as f64);

    // Calculate per-function overhead
    let mut overhead_by_func: std::collections::HashMap<&str, Vec<f64>> = std::collections::HashMap::new();
    for r in &results {
        overhead_by_func.entry(r.name)
            .or_default()
            .push(r.compute_units as f64 / r.rv_instructions as f64);
    }

    println!("Per-Function CU/RV32IM Instruction Ratio:");
    println!("| Function     | Mean CU/Inst | Min       | Max       |");
    println!("|--------------|--------------|-----------|-----------|");
    for (name, ratios) in &overhead_by_func {
        let mean: f64 = ratios.iter().sum::<f64>() / ratios.len() as f64;
        let min = ratios.iter().cloned().fold(f64::INFINITY, f64::min);
        let max = ratios.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        println!("| {:12} | {:>12.2} | {:>9.2} | {:>9.2} |", name, mean, min, max);
    }
    println!();

    println!("Overall Statistics:");
    println!("  Mean SVM/RV32IM instruction expansion: {:.2}x", mean_expansion);
    println!("  Mean Compute Units per RV32IM instruction: {:.2}", mean_cu_per_rv);
    println!();
    println!("Note: Higher CU/inst ratios indicate more memory-bound operations");
    println!("      (loading/storing RISC-V registers from SVM memory).");
}
