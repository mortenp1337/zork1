//! Zork I - Rust Z-Machine Compiler and Interpreter
//! 
//! This program compiles ZIL source files into Z-machine bytecode and runs the game.
//! Usage:
//!   zork1 compile <main.zil>  - Compile ZIL files to z3 format
//!   zork1 run [story.z3]     - Run a compiled story file
//!   zork1                     - Compile and run the default game

mod zmachine;
mod compiler;

use std::fs;
use std::env;
use std::path::Path;

fn main() {
    let args: Vec<String> = env::args().collect();
    
    if args.len() < 2 {
        // Default: try to compile and run
        compile_and_run();
    } else {
        match args[1].as_str() {
            "compile" => {
                if args.len() < 3 {
                    eprintln!("Usage: zork1 compile <main.zil>");
                    std::process::exit(1);
                }
                compile_only(&args[2]);
            }
            "run" => {
                let story = if args.len() > 2 { &args[2] } else { "zork1.z3" };
                run_story(story);
            }
            "help" | "--help" | "-h" => {
                print_help();
            }
            path if path.ends_with(".zil") => {
                compile_only(path);
            }
            path if path.ends_with(".z3") || path.ends_with(".z5") => {
                run_story(path);
            }
            _ => {
                eprintln!("Unknown command: {}", args[1]);
                print_help();
                std::process::exit(1);
            }
        }
    }
}

fn print_help() {
    println!("ZORK I - Rust ZIL Compiler and Z-Machine Interpreter");
    println!();
    println!("Usage:");
    println!("  zork1                     Compile and run the default game");
    println!("  zork1 compile <file.zil>  Compile ZIL source to Z-machine code");
    println!("  zork1 run [file.z3]       Run a compiled story file");
    println!("  zork1 <file.zil>          Compile the specified ZIL file");
    println!("  zork1 <file.z3>           Run the specified story file");
    println!();
    println!("Options:");
    println!("  -h, --help                Show this help message");
}

fn compile_and_run() {
    println!("ZORK I: The Great Underground Empire");
    println!("Rust ZIL Compiler and Z-Machine Interpreter v0.1.0");
    println!();
    
    // Try to find and compile the main ZIL file
    let main_zil = find_main_zil();
    
    match main_zil {
        Some(path) => {
            println!("Compiling {}...", path);
            match compile_zil(&path) {
                Ok(data) => {
                    println!("Compilation successful ({} bytes)", data.len());
                    println!();
                    
                    // Save compiled file
                    let output_path = "zork1.z3";
                    if let Err(e) = fs::write(output_path, &data) {
                        eprintln!("Error writing output file: {}", e);
                        std::process::exit(1);
                    }
                    println!("Output written to {}", output_path);
                    println!();
                    
                    // Run the game
                    run_game(&data);
                }
                Err(e) => {
                    eprintln!("Compilation error: {}", e);
                    std::process::exit(1);
                }
            }
        }
        None => {
            // No ZIL file found, try to run precompiled
            let story = find_story_file();
            match story {
                Some(path) => {
                    println!("Loading precompiled story: {}", path);
                    match fs::read(&path) {
                        Ok(data) => run_game(&data),
                        Err(e) => {
                            eprintln!("Error reading story file: {}", e);
                            std::process::exit(1);
                        }
                    }
                }
                None => {
                    eprintln!("Error: No ZIL source or compiled story found.");
                    eprintln!();
                    eprintln!("To compile, run: zork1 compile zork1.zil");
                    eprintln!("To run precompiled game, place zork1.z3 in current directory.");
                    std::process::exit(1);
                }
            }
        }
    }
}

fn compile_only(path: &str) {
    println!("ZORK I - Rust ZIL Compiler v0.1.0");
    println!();
    println!("Compiling {}...", path);
    
    match compile_zil(path) {
        Ok(data) => {
            // Determine output filename
            let output = if path.ends_with(".zil") {
                path.replace(".zil", ".z3")
            } else {
                format!("{}.z3", path)
            };
            
            if let Err(e) = fs::write(&output, &data) {
                eprintln!("Error writing output file: {}", e);
                std::process::exit(1);
            }
            
            println!("Compilation successful!");
            println!("Output: {} ({} bytes)", output, data.len());
        }
        Err(e) => {
            eprintln!("Compilation error: {}", e);
            std::process::exit(1);
        }
    }
}

fn run_story(path: &str) {
    println!("ZORK I: The Great Underground Empire");
    println!("Rust Z-Machine Interpreter v0.1.0");
    println!();
    
    match fs::read(path) {
        Ok(data) => {
            println!("Loading: {}", path);
            println!();
            run_game(&data);
        }
        Err(e) => {
            eprintln!("Error reading story file: {}", e);
            std::process::exit(1);
        }
    }
}

fn compile_zil(path: &str) -> Result<Vec<u8>, String> {
    let mut compiler = compiler::Compiler::new();
    
    // Change to the directory containing the ZIL file
    let zil_path = Path::new(path);
    if let Some(parent) = zil_path.parent() {
        if !parent.as_os_str().is_empty() {
            std::env::set_current_dir(parent)
                .map_err(|e| format!("Failed to change directory: {}", e))?;
        }
    }
    
    let filename = zil_path.file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(path);
    
    compiler.compile_file(filename)?;
    compiler.generate()
}

fn find_main_zil() -> Option<String> {
    let candidates = ["zork1.zil", "ZORK1.ZIL", "main.zil"];
    
    for name in &candidates {
        if Path::new(name).exists() {
            return Some(name.to_string());
        }
    }
    
    None
}

fn find_story_file() -> Option<String> {
    let locations = [
        "zork1.z3",
        "COMPILED/zork1.z3",
        "target/zork1.z3",
        "zork1.zip",
    ];
    
    for loc in &locations {
        if Path::new(loc).exists() {
            return Some(loc.to_string());
        }
    }
    
    None
}

fn run_game(data: &[u8]) {
    match zmachine::ZMachine::new(data) {
        Ok(mut zm) => {
            if let Err(e) = zm.run() {
                eprintln!("\nGame error: {}", e);
                std::process::exit(1);
            }
        }
        Err(e) => {
            eprintln!("Error initializing Z-Machine: {}", e);
            std::process::exit(1);
        }
    }
}
