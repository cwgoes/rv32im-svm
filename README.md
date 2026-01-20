# rv32im-svm

A RISC-V rv32im instruction set to Solana SVM (sBPF) compiler.

## Overview

This crate provides a compiler that translates RISC-V rv32im instructions to Solana Virtual Machine (sBPF) bytecode. It enables running RISC-V programs compiled from C or other languages on the Solana blockchain.

### Features

- **Full rv32im Support**: All base integer (RV32I) and multiply/divide (M extension) instructions
- **SVM Bytecode Generation**: Produces valid sBPF bytecode compatible with Solana's runtime
- **Built-in Interpreter**: Includes an SVM interpreter for testing and debugging
- **Compute Cost Tracking**: Tracks SVM compute units for cost estimation

## Architecture

```
┌─────────────────┐      ┌─────────────────┐      ┌─────────────────┐
│  RISC-V Binary  │ ───► │    Compiler     │ ───► │  SVM Bytecode   │
│    (rv32im)     │      │                 │      │     (sBPF)      │
└─────────────────┘      └─────────────────┘      └─────────────────┘
```

### Memory Layout

The compiler uses the following memory layout in the SVM:

| Address Range | Description |
|---------------|-------------|
| 0x0000 - 0x7FFF | Program data and heap |
| 0x8000 - 0x807F | RISC-V register file (32 × 4 bytes) |
| 0x8080+ | Stack (grows downward) |

### Register Mapping

RISC-V's 32 registers (x0-x31) are stored in SVM memory at the register file base address. SVM registers (r0-r9) are used as temporaries during instruction execution.

## Usage

### As a Library

```rust
use rv32im_svm::{compile, execute, Instruction, Register};

// Create a simple program: return 42
let instructions = vec![
    Instruction::Addi { rd: Register::A0, rs1: Register::ZERO, imm: 42 },
    Instruction::Ecall,
];

// Compile to SVM
let program = compile(&instructions).unwrap();

// Execute
let result = execute(&program).unwrap();
assert_eq!(result, 42);
```

### From Binary

```rust
use rv32im_svm::{compile_binary, execute};

// Load RISC-V binary (array of 32-bit instruction words)
let binary = vec![0x02a00513, 0x00000073]; // addi a0, zero, 42; ecall

let program = compile_binary(&binary).unwrap();
let result = execute(&program).unwrap();
```

### Command Line

```bash
# Compile and execute a RISC-V binary
cargo run -- program.bin --execute

# Dump decoded instructions
cargo run -- program.bin --dump

# Show compilation statistics
cargo run -- program.bin --stats
```

## Supported Instructions

### RV32I Base Integer Instructions

| Category | Instructions |
|----------|-------------|
| Arithmetic | ADD, SUB, ADDI |
| Logical | AND, OR, XOR, ANDI, ORI, XORI |
| Shift | SLL, SRL, SRA, SLLI, SRLI, SRAI |
| Compare | SLT, SLTU, SLTI, SLTIU |
| Load | LB, LH, LW, LBU, LHU |
| Store | SB, SH, SW |
| Branch | BEQ, BNE, BLT, BGE, BLTU, BGEU |
| Jump | JAL, JALR |
| Upper Imm | LUI, AUIPC |
| System | ECALL, EBREAK, FENCE |

### RV32M Multiply/Divide Extension

| Instruction | Description |
|-------------|-------------|
| MUL | Multiply (lower 32 bits) |
| MULH | Multiply high (signed × signed) |
| MULHSU | Multiply high (signed × unsigned) |
| MULHU | Multiply high (unsigned × unsigned) |
| DIV | Signed division |
| DIVU | Unsigned division |
| REM | Signed remainder |
| REMU | Unsigned remainder |

## Benchmarks

### Mathematical Functions

Compilation statistics for various mathematical functions:

| Function | RV32IM Insts | SVM Insts | Expansion |
|----------|--------------|-----------|-----------|
| factorial | 8 | 68 | 8.50× |
| fibonacci | 13 | 109 | 8.38× |
| gcd | 6 | 66 | 11.00× |
| power | 7 | 59 | 8.43× |
| sum_squares | 9 | 81 | 9.00× |
| isqrt | 8 | 67 | 8.38× |
| modpow | 13 | 146 | 11.23× |

#### Execution Results

| Function | Input | Result | Compute Units |
|----------|-------|--------|---------------|
| factorial | 10 | 3,628,800 | 701 |
| fibonacci | 20 | 6,765 | 1,818 |
| gcd | 1071, 462 | 21 | 365 |
| power | 2^10 | 1,024 | 684 |
| modpow | 7^20 mod 10^9+7 | 868,674,437 | 1,014 |

### SHA-256 Benchmark

Real-world benchmark using SHA-256 compiled from C to rv32im using GCC:

```bash
riscv64-linux-gnu-gcc -march=rv32im -mabi=ilp32 -O2 -c sha256.c
```

#### SHA-256 Statistics

| Metric | Value |
|--------|-------|
| RISC-V Instructions | 263 |
| SVM Instructions | 2,624 |
| Expansion Ratio | 9.98× |
| Compute Units | 108,390 |
| CU per rv32im instruction | 412.13 |

```
SHA-256("abc") first word: 0xba7816bf ✓
```

### Overall SVM Overhead

| Metric | Value |
|--------|-------|
| Mean instruction expansion | 9.31× |
| Mean CU per rv32im instruction | ~65-100 (control flow) |
| SHA-256 CU per rv32im instruction | 412 (memory-intensive) |

The overhead varies based on the instruction mix:
- **Control-flow heavy** (branches, jumps): ~65-80 CU/inst
- **Memory-intensive** (many loads/stores): ~400+ CU/inst

This is because each RISC-V register access requires loading/storing from SVM memory.

## Testing

Run the full test suite:

```bash
cargo test
```

The test suite includes **118 tests** covering:
- All RV32I instructions
- All RV32M instructions
- Edge cases (overflow, division by zero, etc.)
- Complex programs (factorial, GCD, etc.)

## Building

```bash
# Build library
cargo build --release

# Run benchmarks
cargo bench

# Run mathematical functions benchmark
cargo run --example factorial_benchmark

# Run SHA-256 benchmark
cargo run --example sha256_benchmark
```

## Project Structure

```
rv32im-svm/
├── src/
│   ├── lib.rs          # Library entry point
│   ├── main.rs         # CLI tool
│   ├── riscv/
│   │   ├── mod.rs
│   │   ├── instruction.rs  # RISC-V instruction definitions
│   │   └── decoder.rs      # Instruction decoder
│   ├── svm/
│   │   ├── mod.rs
│   │   ├── bytecode.rs     # SVM bytecode definitions
│   │   └── interpreter.rs  # SVM interpreter
│   └── compiler/
│       ├── mod.rs
│       └── translator.rs   # rv32im → SVM translator
├── tests/
│   ├── rv32i_tests.rs  # RV32I instruction tests
│   └── rv32m_tests.rs  # RV32M instruction tests
├── benches/
│   └── factorial.rs    # Factorial benchmark
├── examples/
│   ├── factorial_benchmark.rs   # Mathematical functions benchmark
│   └── sha256_benchmark.rs      # SHA-256 real-world benchmark
└── sha256_benchmark/            # SHA-256 C source files
    ├── sha256.c                 # SHA-256 implementation
    ├── start.S                  # Startup assembly
    └── link.ld                  # Linker script
```

## Limitations

- **No compressed instructions (RVC)**: Only 32-bit instructions are supported
- **No floating-point (F/D extensions)**: Integer operations only
- **Simplified CSR handling**: Most CSRs return 0
- **Memory size**: Limited by SVM memory constraints
- **No dynamic dispatch**: JALR with computed targets has limited support

## License

MIT License - see [LICENSE](LICENSE) for details.
