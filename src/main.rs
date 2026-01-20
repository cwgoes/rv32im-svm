//! rv32im-svm CLI tool

use rv32im_svm::{compile_binary, execute, riscv::decode};
use std::env;
use std::fs::File;
use std::io::{self, Read};

fn main() -> io::Result<()> {
    env_logger::init();

    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: {} <binary-file> [--dump] [--execute]", args[0]);
        eprintln!();
        eprintln!("Options:");
        eprintln!("  --dump      Dump decoded RISC-V instructions");
        eprintln!("  --execute   Execute the compiled program");
        eprintln!("  --stats     Show compilation statistics");
        std::process::exit(1);
    }

    let filename = &args[1];
    let dump = args.contains(&"--dump".to_string());
    let should_execute = args.contains(&"--execute".to_string());
    let stats = args.contains(&"--stats".to_string());

    // Read binary file
    let mut file = File::open(filename)?;
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)?;

    // Convert to 32-bit words (little-endian)
    if buffer.len() % 4 != 0 {
        eprintln!("Error: File size must be a multiple of 4 bytes");
        std::process::exit(1);
    }

    let binary: Vec<u32> = buffer
        .chunks(4)
        .map(|chunk| u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
        .collect();

    if dump {
        println!("Decoded RISC-V instructions:");
        println!("----------------------------");
        for (i, &word) in binary.iter().enumerate() {
            let inst = decode(word);
            println!("{:08x}: {:08x}  {}", i * 4, word, inst);
        }
        println!();
    }

    // Compile
    let program = match compile_binary(&binary) {
        Ok(prog) => prog,
        Err(e) => {
            eprintln!("Compilation error: {}", e);
            std::process::exit(1);
        }
    };

    if stats {
        println!("Compilation statistics:");
        println!("-----------------------");
        println!("RISC-V instructions: {}", binary.len());
        println!("SVM instructions:    {}", program.len());
        println!("Expansion ratio:     {:.2}x", program.len() as f64 / binary.len() as f64);
        println!("Estimated compute:   {} CUs", program.compute_cost());
        println!();
    }

    if should_execute {
        println!("Executing program...");
        match execute(&program) {
            Ok(result) => {
                println!("Result (a0): {}", result);
                println!("Result (hex): {:#x}", result);
            }
            Err(e) => {
                eprintln!("Execution error: {}", e);
                std::process::exit(1);
            }
        }
    }

    Ok(())
}
