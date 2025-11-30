//! ZIL Compiler - Main module
//!
//! This module implements a ZIL (Zork Implementation Language) compiler
//! that compiles ZIL source files to Z-machine bytecode.

pub mod lexer;
pub mod parser;
pub mod ast;
pub mod codegen;
pub mod zcode;
pub mod symbols;

use std::path::Path;
use std::collections::HashMap;

pub use lexer::Lexer;
pub use parser::Parser;
pub use ast::*;
pub use codegen::CodeGenerator;
pub use symbols::SymbolTable;

/// Compiler configuration
pub struct CompilerConfig {
    /// Z-machine version (3 for Zork I)
    pub version: u8,
    /// Whether to include debug information
    pub debug: bool,
}

impl Default for CompilerConfig {
    fn default() -> Self {
        CompilerConfig {
            version: 3,
            debug: false,
        }
    }
}

/// Main compiler struct
pub struct Compiler {
    config: CompilerConfig,
    symbols: SymbolTable,
    source_files: Vec<String>,
}

impl Compiler {
    /// Create a new compiler with default configuration
    pub fn new() -> Self {
        Compiler {
            config: CompilerConfig::default(),
            symbols: SymbolTable::new(),
            source_files: Vec::new(),
        }
    }
    
    /// Create a new compiler with custom configuration
    pub fn with_config(config: CompilerConfig) -> Self {
        Compiler {
            config,
            symbols: SymbolTable::new(),
            source_files: Vec::new(),
        }
    }
    
    /// Compile a ZIL source file
    pub fn compile_file<P: AsRef<Path>>(&mut self, path: P) -> Result<(), String> {
        let path_str = path.as_ref().display().to_string();
        let source = std::fs::read_to_string(path.as_ref())
            .map_err(|e| format!("Failed to read file {}: {}", path_str, e))?;
        
        self.source_files.push(path_str.clone());
        self.compile_source(&source)
            .map_err(|e| format!("{} in file {}", e, path_str))
    }
    
    /// Compile ZIL source code
    pub fn compile_source(&mut self, source: &str) -> Result<(), String> {
        // Lexical analysis
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize()?;
        
        // Parsing
        let mut parser = Parser::new(tokens);
        let forms = parser.parse()?;
        
        // Process each form
        for form in forms {
            self.process_form(form)?;
        }
        
        Ok(())
    }
    
    /// Process a top-level form
    fn process_form(&mut self, form: Form) -> Result<(), String> {
        match form {
            Form::Constant(name, value) => {
                self.symbols.define_constant(&name, value);
            }
            Form::Global(name, value) => {
                self.symbols.define_global(&name, value);
            }
            Form::Object(obj) => {
                self.symbols.define_object(obj);
            }
            Form::Room(room) => {
                self.symbols.define_room(room);
            }
            Form::Routine(routine) => {
                self.symbols.define_routine(routine);
            }
            Form::Syntax(syntax) => {
                self.symbols.define_syntax(syntax);
            }
            Form::InsertFile(filename, _) => {
                // Handle file inclusion
                let path = Path::new(&filename);
                if path.exists() {
                    self.compile_file(path)?;
                } else {
                    // Try with .zil extension
                    let zil_path = format!("{}.zil", filename.to_lowercase());
                    if Path::new(&zil_path).exists() {
                        self.compile_file(&zil_path)?;
                    }
                }
            }
            Form::DefMacro(name, args, body) => {
                self.symbols.define_macro(&name, args, body);
            }
            Form::SetG(name, value) => {
                self.symbols.set_global(&name, value);
            }
            Form::Version(ver) => {
                // Handle version directive
                if ver == "ZIP" {
                    self.config.version = 3;
                }
            }
            Form::Directions(dirs) => {
                self.symbols.define_directions(dirs);
            }
            _ => {
                // Other forms are handled during code generation
            }
        }
        Ok(())
    }
    
    /// Generate Z-machine code
    pub fn generate(&self) -> Result<Vec<u8>, String> {
        let mut codegen = CodeGenerator::new(&self.symbols, self.config.version);
        codegen.generate()
    }
    
    /// Compile and generate Z-machine code in one step
    pub fn build<P: AsRef<Path>>(&mut self, main_file: P) -> Result<Vec<u8>, String> {
        self.compile_file(main_file)?;
        self.generate()
    }
}

impl Default for Compiler {
    fn default() -> Self {
        Self::new()
    }
}
