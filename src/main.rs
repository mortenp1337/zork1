mod lexer;
mod ast;
mod parser;
mod runtime;
mod game;

use std::fs;
use std::path::Path;
use parser::Parser;
use runtime::Runtime;
use game::GameEngine;
use ast::ZilDefinition;
use lexer::{Lexer, Token};

fn test_lexer() {
    // Test with TAB character
    let input = "<OBJECT\tKITCHEN-WINDOW>";
    println!("Testing lexer with TAB: {:?}", input);
    
    let mut lexer = Lexer::new(input);
    loop {
        let token = lexer.next_token();
        println!("Token: {:?}", token);
        if matches!(token, Token::EOF) {
            break;
        }
    }
    println!();
}

fn parse_zil_file(path: &Path) -> Result<Vec<ZilDefinition>, String> {
    let content = fs::read_to_string(path)
        .map_err(|e| format!("Failed to read {}: {}", path.display(), e))?;
    
    let mut parser = Parser::new(&content);
    parser.parse()
}

fn main() {
    // Quick lexer test
    if std::env::args().any(|a| a == "--test-lexer") {
        test_lexer();
        return;
    }
    
    println!("ZIL Compiler for Zork I");
    println!("=======================\n");
    
    // Find the zork1.zil file
    let zil_path = Path::new("zork1.zil");
    
    if !zil_path.exists() {
        eprintln!("Error: zork1.zil not found in current directory");
        std::process::exit(1);
    }
    
    println!("Loading ZIL source files...");
    
    // Load all ZIL files
    let mut definitions = Vec::new();
    
    // Load core files in order
    let core_files = [
        "gmacros.zil",
        "gsyntax.zil",
        "1dungeon.zil",
        "gglobals.zil",
        "gclock.zil",
        "gmain.zil",
        "gparser.zil",
        "gverbs.zil",
        "1actions.zil",
        "zork1.zil",
    ];
    
    for filename in &core_files {
        let path = Path::new(filename);
        if path.exists() {
            eprint!("Loading {}... ", filename);
            match parse_zil_file(path) {
                Ok(defs) => {
                    eprintln!("({} definitions)", defs.len());
                    definitions.extend(defs);
                }
                Err(e) => {
                    eprintln!("Warning: {}", e);
                }
            }
        } else {
            eprintln!("Warning: {} not found", filename);
        }
    }
    
    println!("\nCompiling {} definitions...", definitions.len());
    
    // Count different types
    let mut objects = 0;
    let mut rooms = 0;
    let mut routines = 0;
    let mut globals = 0;
    let mut syntaxes = 0;
    
    for def in &definitions {
        match def {
            ZilDefinition::Object(_) => objects += 1,
            ZilDefinition::Room(_) => rooms += 1,
            ZilDefinition::Routine(_) | ZilDefinition::Macro(_) => routines += 1,
            ZilDefinition::Global(_) | ZilDefinition::Constant(_) => globals += 1,
            ZilDefinition::Syntax(_) => syntaxes += 1,
            _ => {}
        }
    }
    
    println!("  Objects: {}", objects);
    println!("  Rooms: {}", rooms);
    println!("  Routines/Macros: {}", routines);
    println!("  Globals/Constants: {}", globals);
    println!("  Syntax rules: {}", syntaxes);
    
    // Create runtime and load definitions
    let mut runtime = Runtime::new();
    runtime.load_definitions(&definitions);
    
    println!("\nStarting Zork I...\n");
    println!("========================================\n");
    
    // Create and run game engine
    let mut engine = GameEngine::new(runtime);
    engine.init();
    engine.run();
}
