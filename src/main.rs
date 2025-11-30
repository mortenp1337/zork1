//! Zork Interpreter - A Z-machine interpreter for Zork I and other Infocom games
//!
//! This is a Z-machine version 3 interpreter written in Rust that can run
//! classic Infocom games like Zork I, Zork II, Zork III, etc.

mod zmachine;

use std::env;
use std::fs;
use std::process;

use zmachine::ZMachine;

fn main() {
    let args: Vec<String> = env::args().collect();
    
    let story_file = if args.len() > 1 {
        &args[1]
    } else {
        // Default to the bundled Zork I story file
        "COMPILED/zork1.z3"
    };
    
    let story_data = match fs::read(story_file) {
        Ok(data) => data,
        Err(e) => {
            eprintln!("Error: Could not read story file '{}': {}", story_file, e);
            process::exit(1);
        }
    };
    
    let mut zmachine = match ZMachine::new(story_data) {
        Ok(zm) => zm,
        Err(e) => {
            eprintln!("Error: Could not initialize Z-machine: {}", e);
            process::exit(1);
        }
    };
    
    // Run the game
    if let Err(e) = zmachine.run() {
        eprintln!("Error: {}", e);
        process::exit(1);
    }
}
