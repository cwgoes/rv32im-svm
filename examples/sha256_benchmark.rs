//! SHA-256 benchmark
//!
//! This benchmark compiles SHA-256 from C to rv32im (using GCC),
//! then compiles the rv32im binary to SVM and measures compute costs.
//!
//! Expected result: SHA-256("abc") first word = 0xba7816bf

use rv32im_svm::{compile_binary, SvmVm};

/// SHA-256 rv32im code section (263 instructions, 1052 bytes)
/// Compiled from C using riscv64-linux-gnu-gcc -march=rv32im -mabi=ilp32 -O2
fn get_sha256_code() -> Vec<u32> {
    vec![
        0x00007137, 0x00010113, 0x358000ef, 0x00000073, 0xeb010113, 0x14912423, 0x02010493, 0x13612a23,
        0x14812623, 0x15212223, 0x15312023, 0x13412e23, 0x13512c23, 0x13712823, 0x13812623, 0x13912423,
        0x13a12223, 0x13b12023, 0x00050b13, 0x06010813, 0x00048713, 0x0005c783, 0x0015c603, 0x0035c503,
        0x0025c683, 0x01879793, 0x01061613, 0x00c7e7b3, 0x00a7e7b3, 0x00869693, 0x00d7e7b3, 0x00f72023,
        0x00470713, 0x00458593, 0xfd0716e3, 0x04412783, 0x04812f83, 0x04c12f03, 0x05012e83, 0x05412e03,
        0x05812583, 0x05c12303, 0x02012603, 0x02410513, 0x0c448393, 0x00078893, 0x00060293, 0x00052603,
        0x0135d413, 0x00f59713, 0x0115d913, 0x00d59693, 0x00e61793, 0x01265a13, 0x00765813, 0x01961993,
        0x01386833, 0x0086e6b3, 0x0147e7b3, 0x01276733, 0x00a5d413, 0x0107c7b3, 0x00d74733, 0x00365813,
        0x00874733, 0x0107c7b3, 0x00e787b3, 0x011787b3, 0x00030713, 0x00578333, 0x02652e23, 0x00450513,
        0x000f8893, 0x000f0f93, 0x000e8f13, 0x000e0e93, 0x00058e13, 0x00070593, 0xf87510e3, 0x000b2d03,
        0x004b2c83, 0x008b2c03, 0x00cb2b83, 0x014b2e03, 0x010b2403, 0x018b2383, 0x01cb2d83, 0x41c00f13,
        0x51c00a93, 0x000d8793, 0x00038913, 0x000e0993, 0x00040613, 0x000b8a13, 0x000c0e93, 0x000c8f93,
        0x000d0813, 0x01a12623, 0x01912823, 0x01812a23, 0x01712c23, 0x01c12e23, 0x01c0006f, 0x00098913,
        0x000f8e93, 0x00060993, 0x00080f93, 0x00058613, 0x00070813, 0x01a61693, 0x01561713, 0x00665593,
        0x00b65513, 0x00e56533, 0x01965893, 0x00d5e5b3, 0x00761d13, 0x011d6d33, 0x00a5c5b3, 0x01a5c5b3,
        0x0004ae03, 0x000f2d03, 0x01381c13, 0xfff64713, 0x00285c93, 0x01e81313, 0x00d85693, 0x0186e6b3,
        0x01367bb3, 0x01dfc533, 0x01277733, 0x00a81893, 0x01685293, 0x01cd0d33, 0x006ce333, 0x01a58d33,
        0x01dffc33, 0x01057533, 0x01774733, 0x00d346b3, 0x0058e8b3, 0x00ed0733, 0x0116c6b3, 0x01854533,
        0x00f70733, 0x00a686b3, 0x004f0f13, 0x014705b3, 0x00090793, 0x00d70733, 0x000e8a13, 0x00448493,
        0xf35f1ee3, 0x00c12d03, 0x01012c83, 0x01412c03, 0x01812b83, 0x01c12e03, 0x00b40433, 0x008b2823,
        0x14c12403, 0x012d8db3, 0x00ed0d33, 0x010c8cb3, 0x01fc0c33, 0x01db8bb3, 0x013383b3, 0x00ce0e33,
        0x01ab2023, 0x019b2223, 0x018b2423, 0x017b2623, 0x01bb2e23, 0x01cb2a23, 0x007b2c23, 0x14812483,
        0x14412903, 0x14012983, 0x13c12a03, 0x13812a83, 0x13412b03, 0x13012b83, 0x12c12c03, 0x12812c83,
        0x12412d03, 0x12012d83, 0x15010113, 0x00008067, 0x6a09eeb7, 0xbb67be37, 0x3c6ef337, 0xa54ff8b7,
        0x510e5837, 0x9b057637, 0x1f83e6b7, 0x5be0d737, 0x00058793, 0x667e8e93, 0xe85e0e13, 0x37230313,
        0x53a88893, 0x27f80813, 0x88c60613, 0x9ab68693, 0xd1970713, 0x00050593, 0x01d7a023, 0x01c7a223,
        0x0067a423, 0x0117a623, 0x0107a823, 0x00c7aa23, 0x00d7ac23, 0x00e7ae23, 0x00078513, 0xcb5ff06f,
        0xf9010113, 0x02010593, 0x06112623, 0x51c00793, 0x00058713, 0x55c00693, 0x0007a883, 0x0047a803,
        0x0087a503, 0x00c7a603, 0x01172023, 0x01072223, 0x00a72423, 0x00c72623, 0x01078793, 0x01070713,
        0xfcd79ce3, 0x6a09ee37, 0xbb67b337, 0x3c6ef8b7, 0xa54ff837, 0x510e5637, 0x9b0576b7, 0x1f83e737,
        0x5be0d7b7, 0x00010513, 0x667e0e13, 0xe8530313, 0x37288893, 0x53a80813, 0x27f60613, 0x88c68693,
        0x9ab70713, 0xd1978793, 0x01c12023, 0x00612223, 0x01112423, 0x01012623, 0x00c12823, 0x00d12a23,
        0x00e12c23, 0x00f12e23, 0xc09ff0ef, 0x06c12083, 0x00012503, 0x07010113, 0x00008067,
    ]
}

/// SHA-256 data section (80 words, 320 bytes)
/// Contains k constants (64 words) and input block for "abc" (16 words)
fn get_sha256_data() -> Vec<u32> {
    vec![
        // k constants (64 words)
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5,
        0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174,
        0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
        0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7, 0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967,
        0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85,
        0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
        0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
        0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2,
        // Input block for "abc" (16 words) - stored as bytes read by lbu instructions
        0x80636261, 0x00000000, 0x00000000, 0x00000000, 0x00000000, 0x00000000, 0x00000000, 0x00000000,
        0x00000000, 0x00000000, 0x00000000, 0x00000000, 0x00000000, 0x00000000, 0x00000000, 0x18000000,
    ]
}

fn main() {
    println!("=== SHA-256 SVM Benchmark ===\n");

    // Get the RISC-V code and data
    let code = get_sha256_code();
    let data = get_sha256_data();
    let rv_instructions = code.len();
    let data_words = data.len();

    println!("Input Statistics:");
    println!("  RISC-V code: {} instructions ({} bytes)", rv_instructions, rv_instructions * 4);
    println!("  Data section: {} words ({} bytes)", data_words, data_words * 4);

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
    println!("\nExecuting SHA-256(\"abc\")...");

    let mut vm = SvmVm::new(0x20000); // 128KB memory
    vm.set_compute_limit(1_000_000_000); // 1B compute units max

    // Load data section at address 0x41c
    let data_base = 0x41c;
    for (i, &word) in data.iter().enumerate() {
        let addr = data_base + i * 4;
        let bytes = word.to_le_bytes();
        vm.memory[addr..addr + 4].copy_from_slice(&bytes);
    }

    let result = match vm.execute(&program) {
        Ok(r) => r,
        Err(e) => {
            println!("Execution failed: {:?}", e);
            println!("Compute units used: {}", vm.compute_units);
            return;
        }
    };

    // Truncate to 32 bits (RV32 result in 64-bit register)
    let result_u32 = result as u32;
    println!("\nResults:");
    println!("  SHA-256(\"abc\") first word: 0x{:08x}", result_u32);
    println!("  Expected:                   0xba7816bf");
    println!("  Match: {}", if result_u32 == 0xba7816bf { "YES" } else { "NO" });
    println!("\n  Total Compute Units: {}", vm.compute_units);
    println!("  CU per rv32im instruction: {:.2}", vm.compute_units as f64 / rv_instructions as f64);

    // Multiple runs for statistics
    println!("\n=== Multiple Run Statistics ===\n");
    println!("Running SHA-256 hash 5 times...");

    let mut total_cu = 0u64;
    for i in 0..5 {
        let mut vm = SvmVm::new(0x20000);
        vm.set_compute_limit(1_000_000_000);

        // Load data section
        for (j, &word) in data.iter().enumerate() {
            let addr = data_base + j * 4;
            let bytes = word.to_le_bytes();
            vm.memory[addr..addr + 4].copy_from_slice(&bytes);
        }

        let result = vm.execute(&program).expect("Execution failed") as u32;
        total_cu += vm.compute_units;
        println!("  Run {}: result=0x{:08x}, CU={}", i + 1, result, vm.compute_units);
    }

    println!("\n  Average Compute Units: {}", total_cu / 5);
    println!("  Average CU per rv32im instruction: {:.2}", (total_cu / 5) as f64 / rv_instructions as f64);
}
