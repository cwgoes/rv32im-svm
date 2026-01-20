//! Meta-Compiler Benchmark
//!
//! This benchmark compiles a minimal rv32im compiler from C to rv32im,
//! then from rv32im to SVM, and measures the performance of running
//! the meta-compiled compiler to generate factorial code.
//!
//! The mini compiler:
//! - Encodes rv32im instructions (R-type, I-type, B-type formats)
//! - Compiles factorial(10) to rv32im machine code
//! - Verifies the output and computes a checksum
//! - Runs 100 iterations for measurable performance
//!
//! Expected result format: upper 16 bits = total instructions (800), lower 16 bits = checksum

use rv32im_svm::{compile_binary, SvmVm};

/// Mini compiler rv32im code (55 instructions, 220 bytes)
/// Compiled from C using riscv64-linux-gnu-gcc -march=rv32im -mabi=ilp32 -O2
///
/// The compiler implements:
/// - encode_r: R-type instruction encoding
/// - encode_i: I-type instruction encoding
/// - encode_b: B-type instruction encoding
/// - addi, add, mul, blt, ecall instruction generators
/// - compile_factorial: generates factorial(10) code
/// - verify_output: validates generated instructions
/// - compute_checksum: computes XOR checksum
/// - mini_compiler_benchmark: runs 100 compilation iterations
fn get_mini_compiler_code() -> Vec<u32> {
    vec![
        // _start (4 instructions)
        0x00007137, // lui sp, 0x7
        0x00010113, // addi sp, sp, 0
        0x008000ef, // jal ra, mini_compiler_benchmark
        0x00000073, // ecall
        // mini_compiler_benchmark (51 instructions)
        0xff010113, // addi sp, sp, -16
        0x11000613, // addi a2, zero, 272  (output buffer address)
        0x00a002b7, // lui t0, 0xa00
        0x00100fb7, // lui t6, 0x100
        0x00200f37, // lui t5, 0x200
        0x00c5deb7, // lui t4, 0xc5d
        0x02b50e37, // lui t3, 0x2b50
        0xfff58337, // lui t1, 0xfff58
        0xfec058b7, // lui a7, 0xfec05
        0x00812623, // sw s0, 12(sp)
        0x06400513, // addi a0, zero, 100  (loop counter)
        0x00000813, // addi a6, zero, 0    (checksum)
        0x13000593, // addi a1, zero, 304  (output end)
        0x59328293, // addi t0, t0, 1427   (addi a1, zero, 10)
        0x513f8f93, // addi t6, t6, 1299   (addi a0, zero, 1)
        0x613f0f13, // addi t5, t5, 1555   (addi a2, zero, 2)
        0x863e8e93, // addi t4, t4, -1949  (blt a1, a2, 16)
        0x533e0e13, // addi t3, t3, 1331   (mul a0, a0, a1)
        0x59330313, // addi t1, t1, 1427   (addi a1, a1, -1)
        0xae388893, // addi a7, a7, -1309  (blt zero, a2, -12)
        0x07300413, // addi s0, zero, 115  (ecall = 0x73)
        0x00060393, // addi t2, a2, 0      (save output base)
        // Store 8 instructions to output buffer
        0x00562023, // sw t0, 0(a2)
        0x01f62223, // sw t6, 4(a2)
        0x01e62423, // sw t5, 8(a2)
        0x01d62623, // sw t4, 12(a2)
        0x01c62823, // sw t3, 16(a2)
        0x00662a23, // sw t1, 20(a2)
        0x01162c23, // sw a7, 24(a2)
        0x00862e23, // sw s0, 28(a2)
        // Checksum computation loop
        0x11000693, // addi a3, zero, 272  (reset to output base)
        0x00038613, // addi a2, t2, 0
        0x00000713, // addi a4, zero, 0    (sum = 0)
        0x0006a783, // lw a5, 0(a3)
        0x00468693, // addi a3, a3, 4
        0x00f70733, // add a4, a4, a5
        0x0107d793, // srli a5, a5, 16
        0x00e7c733, // xor a4, a5, a4
        0xfeb696e3, // bne a3, a1, -20
        0xfff50513, // addi a0, a0, -1     (decrement counter)
        0x00e84833, // xor a6, a6, a4      (accumulate checksum)
        0xfa051ae3, // bne a0, zero, -92   (outer loop)
        // Epilogue
        0x00c12403, // lw s0, 12(sp)
        0x01081813, // slli a6, a6, 16
        0x00800793, // addi a5, zero, 8    (output_len = 8)
        0x01085813, // srli a6, a6, 16
        0x10f3a023, // sw a5, 256(t2)      (store output_len)
        0x03200537, // lui a0, 0x3200      (upper bits: 800 << 16)
        0x00a86533, // or a0, a6, a0       (combine with checksum)
        0x01010113, // addi sp, sp, 16
        0x00008067, // jalr zero, 0(ra)    (return)
    ]
}

fn main() {
    println!("=== Meta-Compiler SVM Benchmark ===\n");
    println!("Running a compiler compiled to rv32im, then to SVM,");
    println!("to compile factorial(10) to rv32im machine code.\n");

    // Get the RISC-V code
    let code = get_mini_compiler_code();
    let rv_instructions = code.len();

    println!("Input Statistics:");
    println!("  Mini-compiler RISC-V instructions: {}", rv_instructions);
    println!("  Mini-compiler binary size: {} bytes", rv_instructions * 4);

    // Compile code to SVM
    println!("\nCompiling rv32im to SVM...");
    let program = match compile_binary(&code) {
        Ok(p) => p,
        Err(e) => {
            println!("Compilation failed: {:?}", e);
            return;
        }
    };

    let svm_instructions = program.len();
    let expansion_ratio = svm_instructions as f64 / rv_instructions as f64;

    println!("Compilation Statistics:");
    println!("  SVM instructions: {}", svm_instructions);
    println!("  Expansion ratio: {:.2}x", expansion_ratio);

    // Execute
    println!("\nExecuting meta-compiled compiler...");
    println!("(Compiling factorial 100 times and computing checksum)");

    let mut vm = SvmVm::new(0x10000); // 64KB memory
    vm.set_compute_limit(100_000_000); // 100M compute units max

    let result = match vm.execute(&program) {
        Ok(r) => r,
        Err(e) => {
            println!("Execution failed: {:?}", e);
            println!("Compute units used: {}", vm.compute_units);
            return;
        }
    };

    // Parse result: upper 16 bits = total instructions, lower 16 bits = checksum
    let result_u32 = result as u32;
    let total_instructions = (result_u32 >> 16) as u32;
    let checksum = (result_u32 & 0xFFFF) as u32;

    println!("\nResults:");
    println!("  Total instructions compiled: {} (expected: 800 = 8 * 100)", total_instructions);
    println!("  Checksum: 0x{:04x}", checksum);
    println!("\n  Total Compute Units: {}", vm.compute_units);
    println!("  CU per mini-compiler rv32im instruction: {:.2}",
             vm.compute_units as f64 / rv_instructions as f64);

    // Verify the generated factorial code
    println!("\n=== Verifying Generated Code ===\n");

    // Read the output buffer from VM memory (address 0x110 = 272)
    let output_base = 0x110; // 272 in hex
    let mut generated_code: Vec<u32> = Vec::new();
    for i in 0..8 {
        let addr = output_base + i * 4;
        let word = u32::from_le_bytes([
            vm.memory[addr],
            vm.memory[addr + 1],
            vm.memory[addr + 2],
            vm.memory[addr + 3],
        ]);
        generated_code.push(word);
    }

    println!("Generated factorial rv32im code:");
    let expected_names = [
        "addi a1, zero, 10",
        "addi a0, zero, 1",
        "addi a2, zero, 2",
        "blt a1, a2, 16",
        "mul a0, a0, a1",
        "addi a1, a1, -1",
        "blt zero, a2, -12",
        "ecall",
    ];
    let expected_encodings: [u32; 8] = [
        0x00a00593, // addi a1, zero, 10
        0x00100513, // addi a0, zero, 1
        0x00200613, // addi a2, zero, 2
        0x00c5c863, // blt a1, a2, 16
        0x02b50533, // mul a0, a0, a1
        0xfff58593, // addi a1, a1, -1
        0xfec04ae3, // blt zero, a2, -12 (always branches since 0 < 2)
        0x00000073, // ecall
    ];

    let mut all_correct = true;
    for (i, (generated, expected)) in generated_code.iter().zip(expected_encodings.iter()).enumerate() {
        let status = if *generated == *expected { "OK" } else { "MISMATCH" };
        if *generated != *expected {
            all_correct = false;
        }
        println!("  [{}] 0x{:08x} {} (expected 0x{:08x}) [{}]",
                 i, generated, expected_names[i], expected, status);
    }

    println!("\nCode verification: {}", if all_correct { "PASSED" } else { "FAILED" });

    // Run the generated factorial code
    println!("\n=== Running Generated Factorial Code ===\n");

    let factorial_program = match compile_binary(&generated_code) {
        Ok(p) => p,
        Err(e) => {
            println!("Failed to compile generated code: {:?}", e);
            return;
        }
    };

    let mut factorial_vm = SvmVm::new(0x10000);
    factorial_vm.set_compute_limit(10_000_000);

    let factorial_result = match factorial_vm.execute(&factorial_program) {
        Ok(r) => r,
        Err(e) => {
            println!("Factorial execution failed: {:?}", e);
            return;
        }
    };

    println!("factorial(10) = {} (expected: 3628800)", factorial_result);
    println!("Compute units: {}", factorial_vm.compute_units);

    // Summary
    println!("\n=== Summary ===\n");
    println!("Meta-compilation pipeline:");
    println!("  1. C compiler (mini_compiler.c)");
    println!("     -> GCC rv32im cross-compiler");
    println!("  2. rv32im binary ({} instructions)", rv_instructions);
    println!("     -> rv32im-svm compiler");
    println!("  3. SVM bytecode ({} instructions)", svm_instructions);
    println!("     -> SVM interpreter");
    println!("  4. Compiled factorial(10) to 8 rv32im instructions");
    println!("\nPerformance:");
    println!("  Meta-compiler CU: {}", vm.compute_units);
    println!("  CU per source rv32im instruction: {:.2}",
             vm.compute_units as f64 / rv_instructions as f64);
    println!("  Generated factorial(10) = {}", factorial_result);
}
