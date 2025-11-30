//! Zork I - Rust Z-Machine Interpreter
//! 
//! This program runs the classic Zork I text adventure game using a Rust-based
//! Z-Machine interpreter. The game can be compiled from ZIL source files using
//! the zork1-build tool.

mod zmachine;

use std::fs;
use std::env;
use std::path::Path;

fn main() {
    println!("ZORK I: The Great Underground Empire");
    println!("Copyright (c) 1980 Infocom, Inc. All rights reserved.");
    println!("Rust Z-Machine Interpreter v0.1.0");
    println!();
    
    // Find the story file
    let story_path = find_story_file();
    
    match story_path {
        Some(path) => {
            println!("Loading story file: {}", path);
            println!();
            
            match fs::read(&path) {
                Ok(data) => run_game(&data),
                Err(e) => {
                    eprintln!("Error reading story file: {}", e);
                    std::process::exit(1);
                }
            }
        }
        None => {
            eprintln!("Error: No story file found.");
            eprintln!();
            eprintln!("Usage: zork1 [story-file.z3]");
            eprintln!();
            eprintln!("Or place a story file in one of these locations:");
            eprintln!("  - ./zork1.z3");
            eprintln!("  - ./COMPILED/zork1.z3");
            eprintln!("  - ./target/zork1.z3");
            std::process::exit(1);
        }
    }
}

/// Find the story file in various locations
fn find_story_file() -> Option<String> {
    // Check command line arguments
    let args: Vec<String> = env::args().collect();
    if args.len() > 1 {
        let path = &args[1];
        if Path::new(path).exists() {
            return Some(path.clone());
        }
    }
    
    // Check standard locations
    let locations = [
        "zork1.z3",
        "COMPILED/zork1.z3",
        "target/zork1.z3",
        "../COMPILED/zork1.z3",
    ];
    
    for loc in &locations {
        if Path::new(loc).exists() {
            return Some(loc.to_string());
        }
    }
    
    // Also check for .zip extension (some z3 files are named .zip)
    if Path::new("zork1.zip").exists() {
        return Some("zork1.zip".to_string());
    }
    
    None
}

/// Run the game with the given story data
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
