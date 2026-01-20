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

### Factorial Function

The factorial function compiled from C:

```c
int factorial(int n) {
    int result = 1;
    while (n > 1) {
        result *= n;
        n--;
    }
    return result;
}
```

#### Compilation Statistics

| Metric | Value |
|--------|-------|
| RISC-V Instructions | 8 |
| SVM Instructions | 68 |
| Expansion Ratio | 8.50× |
| Base Compute Cost | 133 CUs |

#### Execution Results

| n | factorial(n) | Compute Units |
|---|--------------|---------------|
| 3 | 6 | 211 |
| 5 | 120 | 351 |
| 7 | 5,040 | 491 |
| 10 | 3,628,800 | 701 |
| 12 | 479,001,600 | 841 |

#### Compute Cost Breakdown

Each loop iteration consumes approximately **70 compute units**:
- Register load/store operations: ~40 CUs
- Branch evaluation: ~10 CUs
- Multiplication: 10 CUs
- Arithmetic operations: ~10 CUs

### Cost Formula

For factorial(n), the approximate compute cost is:

```
CU ≈ 133 + 70 × (n - 1)
```

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

# Run example
cargo run --example factorial_benchmark
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
└── examples/
    └── factorial_benchmark.rs
```

## Limitations

- **No compressed instructions (RVC)**: Only 32-bit instructions are supported
- **No floating-point (F/D extensions)**: Integer operations only
- **Simplified CSR handling**: Most CSRs return 0
- **Memory size**: Limited by SVM memory constraints
- **No dynamic dispatch**: JALR with computed targets has limited support

## License

MIT License - see [LICENSE](LICENSE) for details.
