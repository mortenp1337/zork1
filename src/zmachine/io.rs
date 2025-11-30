//! I/O handling for the Z-machine
//!
//! Handles input and output operations for the interpreter.

use std::io::{self, Write, BufRead};
use super::ZMachineError;

/// IOSystem handles all input/output for the Z-machine
pub struct IOSystem {
    output_buffer: String,
}

impl IOSystem {
    /// Create a new IO system
    pub fn new() -> Self {
        IOSystem {
            output_buffer: String::new(),
        }
    }
    
    /// Print text to the output
    pub fn print(&mut self, text: &str) {
        // Buffer output for efficiency
        self.output_buffer.push_str(text);
        
        // Flush on newlines
        if text.contains('\n') {
            self.flush();
        }
    }
    
    /// Flush buffered output
    pub fn flush(&mut self) {
        if !self.output_buffer.is_empty() {
            print!("{}", self.output_buffer);
            let _ = io::stdout().flush();
            self.output_buffer.clear();
        }
    }
    
    /// Read a line of input from the user
    pub fn read_line(&mut self) -> Result<String, ZMachineError> {
        self.flush();
        
        print!("> ");
        let _ = io::stdout().flush();
        
        let stdin = io::stdin();
        let mut line = String::new();
        
        match stdin.lock().read_line(&mut line) {
            Ok(_) => {
                // Remove trailing newline
                line = line.trim_end().to_string();
                Ok(line)
            }
            Err(e) => Err(ZMachineError::new(format!("Input error: {}", e))),
        }
    }
}

impl Default for IOSystem {
    fn default() -> Self {
        Self::new()
    }
}
