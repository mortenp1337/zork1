//! Z-Machine interpreter for running Infocom-era text adventure games.
//! 
//! This module implements a Z-Machine interpreter that can run Z-code version 3
//! story files, such as Zork I.

mod memory;
mod instruction;
mod opcodes;

pub use memory::Memory;
pub use instruction::{Instruction, Opcode, OperandType};
pub use opcodes::Frame;

use std::io::{self, Write, BufRead};

/// The Z-Machine interpreter state
pub struct ZMachine {
    /// Z-machine memory
    pub memory: Memory,
    /// Program counter
    pub pc: usize,
    /// Call stack frames
    pub frames: Vec<Frame>,
    /// Running flag
    pub running: bool,
    /// Output buffer
    output_buffer: String,
}

impl ZMachine {
    /// Create a new Z-Machine with the given story data
    pub fn new(story_data: &[u8]) -> Result<Self, String> {
        let memory = Memory::new(story_data)?;
        
        // Get initial PC from header (bytes 6-7)
        let initial_pc = memory.read_word(0x06) as usize;
        
        // Create initial frame
        let initial_frame = Frame {
            return_pc: 0,
            locals: Vec::new(),
            stack: Vec::new(),
            store_var: None,
        };
        
        Ok(ZMachine {
            memory,
            pc: initial_pc,
            frames: vec![initial_frame],
            running: true,
            output_buffer: String::new(),
        })
    }
    
    /// Run the Z-Machine interpreter
    pub fn run(&mut self) -> Result<(), String> {
        let mut step_count: u64 = 0;
        let debug = std::env::var("ZORK_DEBUG").is_ok();
        while self.running {
            step_count += 1;
            if step_count > 10_000_000 {
                // Prevent infinite loops during development
                return Err("Maximum step count exceeded".to_string());
            }
            if debug && step_count <= 100 {
                eprintln!("Step {}: PC=0x{:04X}", step_count, self.pc);
            }
            self.step()?;
        }
        Ok(())
    }
    
    /// Execute a single instruction
    fn step(&mut self) -> Result<(), String> {
        let instruction = self.decode_instruction()?;
        self.execute_instruction(&instruction)?;
        Ok(())
    }
    
    /// Decode the instruction at the current PC
    fn decode_instruction(&mut self) -> Result<Instruction, String> {
        let opcode_byte = self.read_byte();
        
        let (opcode, operands) = if opcode_byte == 0xBE {
            // Extended opcode
            let ext_opcode = self.read_byte();
            let operand_types = self.read_operand_types(2);
            let operands = self.read_operands(&operand_types);
            (Opcode::Extended(ext_opcode), operands)
        } else if opcode_byte & 0xC0 == 0xC0 {
            // Variable form
            let is_2op = (opcode_byte & 0x20) == 0;
            let opcode_num = opcode_byte & 0x1F;
            // VAR form: read 1 byte of operand types (up to 4 operands)
            // Special opcodes (call_vs2, call_vn2) read 2 bytes for up to 8 operands
            let is_double_var = (opcode_num == 0x0C || opcode_num == 0x1A) && !is_2op;
            let operand_types = self.read_operand_types(if is_double_var { 2 } else { 1 });
            let operands = self.read_operands(&operand_types);
            if is_2op {
                (Opcode::TwoOp(opcode_num), operands)
            } else {
                (Opcode::Var(opcode_num, 0), operands)
            }
        } else if opcode_byte & 0x80 == 0x80 {
            // Short form (1OP or 0OP)
            let op_type = (opcode_byte >> 4) & 0x03;
            let opcode_num = opcode_byte & 0x0F;
            if op_type == 3 {
                // 0OP
                (Opcode::ZeroOp(opcode_num), vec![])
            } else {
                // 1OP
                let operand_type = match op_type {
                    0 => OperandType::LargeConstant,
                    1 => OperandType::SmallConstant,
                    2 => OperandType::Variable,
                    _ => OperandType::Omitted,
                };
                let operands = self.read_operands(&[operand_type]);
                (Opcode::OneOp(opcode_num), operands)
            }
        } else {
            // Long form (2OP)
            let opcode_num = opcode_byte & 0x1F;
            let op1_type = if opcode_byte & 0x40 == 0 { OperandType::SmallConstant } else { OperandType::Variable };
            let op2_type = if opcode_byte & 0x20 == 0 { OperandType::SmallConstant } else { OperandType::Variable };
            let operands = self.read_operands(&[op1_type, op2_type]);
            (Opcode::TwoOp(opcode_num), operands)
        };
        
        Ok(Instruction { opcode, operands })
    }
    
    /// Read operand types from bytes
    fn read_operand_types(&mut self, byte_count: usize) -> Vec<OperandType> {
        let mut types = Vec::new();
        for _ in 0..byte_count {
            let type_byte = self.read_byte();
            for i in (0..4).rev() {
                let op_type = (type_byte >> (i * 2)) & 0x03;
                match op_type {
                    0 => types.push(OperandType::LargeConstant),
                    1 => types.push(OperandType::SmallConstant),
                    2 => types.push(OperandType::Variable),
                    _ => break, // 3 = omitted, marks end
                }
            }
        }
        types
    }
    
    /// Read operands based on their types
    fn read_operands(&mut self, types: &[OperandType]) -> Vec<u16> {
        let mut operands = Vec::new();
        for op_type in types {
            match op_type {
                OperandType::LargeConstant => {
                    operands.push(self.read_word());
                }
                OperandType::SmallConstant => {
                    operands.push(self.read_byte() as u16);
                }
                OperandType::Variable => {
                    let var_num = self.read_byte();
                    operands.push(self.get_variable(var_num));
                }
                OperandType::Omitted => {}
            }
        }
        operands
    }
    
    /// Read a byte from memory and advance PC
    fn read_byte(&mut self) -> u8 {
        let byte = self.memory.read_byte(self.pc);
        self.pc += 1;
        byte
    }
    
    /// Read a word from memory and advance PC
    fn read_word(&mut self) -> u16 {
        let word = self.memory.read_word(self.pc);
        self.pc += 2;
        word
    }
    
    /// Get a variable's value
    fn get_variable(&mut self, var_num: u8) -> u16 {
        if var_num == 0 {
            // Pop from stack
            self.frames.last_mut().unwrap().stack.pop().unwrap_or(0)
        } else if var_num <= 15 {
            // Local variable
            let frame = self.frames.last().unwrap();
            frame.locals.get((var_num - 1) as usize).copied().unwrap_or(0)
        } else {
            // Global variable
            let globals_addr = self.memory.read_word(0x0C) as usize;
            let var_addr = globals_addr + ((var_num as usize - 16) * 2);
            self.memory.read_word(var_addr)
        }
    }
    
    /// Set a variable's value
    fn set_variable(&mut self, var_num: u8, value: u16) {
        if var_num == 0 {
            // Push to stack
            self.frames.last_mut().unwrap().stack.push(value);
        } else if var_num <= 15 {
            // Local variable
            let frame = self.frames.last_mut().unwrap();
            while frame.locals.len() < var_num as usize {
                frame.locals.push(0);
            }
            frame.locals[(var_num - 1) as usize] = value;
        } else {
            // Global variable
            let globals_addr = self.memory.read_word(0x0C) as usize;
            let var_addr = globals_addr + ((var_num as usize - 16) * 2);
            self.memory.write_word(var_addr, value);
        }
    }
    
    /// Store result to a variable
    fn store_result(&mut self, value: u16) {
        let var_num = self.read_byte();
        self.set_variable(var_num, value);
    }
    
    /// Execute a branch instruction
    fn branch(&mut self, condition: bool) {
        let branch_byte = self.read_byte();
        let branch_on_true = (branch_byte & 0x80) != 0;
        let long_branch = (branch_byte & 0x40) == 0;
        
        let offset = if long_branch {
            let next_byte = self.read_byte();
            let offset = (((branch_byte & 0x3F) as u16) << 8) | (next_byte as u16);
            // Sign extend from 14 bits
            if offset & 0x2000 != 0 {
                (offset | 0xC000) as i16
            } else {
                offset as i16
            }
        } else {
            (branch_byte & 0x3F) as i16
        };
        
        if condition == branch_on_true {
            match offset {
                0 => {
                    // Return false
                    self.return_from_routine(0);
                }
                1 => {
                    // Return true
                    self.return_from_routine(1);
                }
                _ => {
                    // Branch to offset
                    self.pc = ((self.pc as i32) + (offset as i32) - 2) as usize;
                }
            }
        }
    }
    
    /// Call a routine
    fn call_routine(&mut self, packed_addr: u16, args: &[u16], store_var: Option<u8>) {
        if packed_addr == 0 {
            // Call to address 0 returns 0
            if let Some(var) = store_var {
                self.set_variable(var, 0);
            }
            return;
        }
        
        // Convert packed address to byte address (for v3: multiply by 2)
        let routine_addr = (packed_addr as usize) * 2;
        
        // Read number of local variables
        let num_locals = self.memory.read_byte(routine_addr);
        
        // Create new frame
        let mut frame = Frame {
            return_pc: self.pc,
            locals: Vec::with_capacity(num_locals as usize),
            stack: Vec::new(),
            store_var,
        };
        
        // Initialize local variables (v3: read default values from routine header)
        let version = self.memory.read_byte(0x00);
        if version <= 4 {
            for i in 0..num_locals {
                let default_val = self.memory.read_word(routine_addr + 1 + (i as usize * 2));
                frame.locals.push(default_val);
            }
            self.pc = routine_addr + 1 + (num_locals as usize * 2);
        } else {
            for _ in 0..num_locals {
                frame.locals.push(0);
            }
            self.pc = routine_addr + 1;
        }
        
        // Copy arguments to locals
        for (i, &arg) in args.iter().enumerate() {
            if i < frame.locals.len() {
                frame.locals[i] = arg;
            }
        }
        
        self.frames.push(frame);
    }
    
    /// Return from a routine
    fn return_from_routine(&mut self, value: u16) {
        if self.frames.len() <= 1 {
            self.running = false;
            return;
        }
        
        let frame = self.frames.pop().unwrap();
        self.pc = frame.return_pc;
        
        if let Some(var) = frame.store_var {
            self.set_variable(var, value);
        }
    }
    
    /// Print a ZSCII string from the given address
    fn print_zstring(&mut self, addr: usize) -> usize {
        let text = self.decode_zstring(addr);
        self.print(&text);
        self.get_zstring_length(addr)
    }
    
    /// Get the length of a Z-string in bytes
    fn get_zstring_length(&self, addr: usize) -> usize {
        let mut offset = 0;
        loop {
            let word = self.memory.read_word(addr + offset);
            offset += 2;
            if word & 0x8000 != 0 {
                break;
            }
        }
        offset
    }
    
    /// Decode a Z-string from memory
    fn decode_zstring(&self, addr: usize) -> String {
        let mut result = String::new();
        let mut offset = 0;
        let mut alphabet = 0u8;
        let mut abbrev_mode = 0u8;
        let mut zscii_high = 0u8;
        let mut zscii_mode = 0u8;
        
        loop {
            let word = self.memory.read_word(addr + offset);
            offset += 2;
            let end_bit = word & 0x8000 != 0;
            
            let chars = [
                ((word >> 10) & 0x1F) as u8,
                ((word >> 5) & 0x1F) as u8,
                (word & 0x1F) as u8,
            ];
            
            for zchar in chars {
                if abbrev_mode > 0 {
                    // Abbreviation lookup
                    let abbrev_table = self.memory.read_word(0x18) as usize;
                    let abbrev_num = ((abbrev_mode - 1) as usize * 32) + zchar as usize;
                    let word_addr = self.memory.read_word(abbrev_table + abbrev_num * 2) as usize;
                    let abbrev_addr = word_addr * 2;
                    result.push_str(&self.decode_zstring(abbrev_addr));
                    abbrev_mode = 0;
                } else if zscii_mode == 1 {
                    zscii_high = zchar;
                    zscii_mode = 2;
                } else if zscii_mode == 2 {
                    let zscii_char = ((zscii_high as u16) << 5) | (zchar as u16);
                    if zscii_char >= 32 && zscii_char <= 126 {
                        result.push(zscii_char as u8 as char);
                    }
                    zscii_mode = 0;
                    alphabet = 0;
                } else {
                    match zchar {
                        0 => result.push(' '),
                        1..=3 => abbrev_mode = zchar,
                        4 => alphabet = 1,
                        5 => alphabet = 2,
                        6 if alphabet == 2 => {
                            // ZSCII escape
                            zscii_mode = 1;
                        }
                        _ => {
                            let char_index = zchar as usize - 6;
                            let alphabets: [&str; 3] = [
                                "abcdefghijklmnopqrstuvwxyz",
                                "ABCDEFGHIJKLMNOPQRSTUVWXYZ",
                                " \n0123456789.,!?_#'\"/\\-:()",
                            ];
                            if char_index < 26 {
                                if let Some(c) = alphabets[alphabet as usize].chars().nth(char_index) {
                                    result.push(c);
                                }
                            }
                            alphabet = 0;
                        }
                    }
                }
            }
            
            if end_bit {
                break;
            }
        }
        
        result
    }
    
    /// Print text to output
    fn print(&mut self, text: &str) {
        print!("{}", text);
        io::stdout().flush().ok();
        self.output_buffer.push_str(text);
    }
    
    /// Print a newline
    fn print_newline(&mut self) {
        println!();
        self.output_buffer.push('\n');
    }
    
    /// Read a line of input from the user
    fn read_line(&mut self, text_buffer_addr: usize, parse_buffer_addr: usize) {
        // Flush output
        io::stdout().flush().ok();
        
        // Read input
        let mut input = String::new();
        io::stdin().lock().read_line(&mut input).ok();
        let input = input.trim().to_lowercase();
        
        // Store in text buffer (v3 format)
        let max_len = self.memory.read_byte(text_buffer_addr) as usize;
        let input_bytes: Vec<u8> = input.bytes().take(max_len.saturating_sub(1)).collect();
        
        for (i, &byte) in input_bytes.iter().enumerate() {
            self.memory.write_byte(text_buffer_addr + 1 + i, byte);
        }
        // Null terminate
        self.memory.write_byte(text_buffer_addr + 1 + input_bytes.len(), 0);
        
        // Parse input into tokens (tokenize)
        self.tokenize(&input, parse_buffer_addr);
    }
    
    /// Tokenize input text
    fn tokenize(&mut self, input: &str, parse_buffer_addr: usize) {
        let max_tokens = self.memory.read_byte(parse_buffer_addr) as usize;
        let dict_addr = self.memory.read_word(0x08) as usize;
        
        // Parse dictionary header
        let num_separators = self.memory.read_byte(dict_addr) as usize;
        let mut separators = Vec::new();
        for i in 0..num_separators {
            separators.push(self.memory.read_byte(dict_addr + 1 + i) as char);
        }
        
        let entry_length = self.memory.read_byte(dict_addr + 1 + num_separators) as usize;
        let num_entries = self.memory.read_word(dict_addr + 2 + num_separators) as usize;
        let entries_start = dict_addr + 4 + num_separators;
        
        // Tokenize input
        let mut tokens: Vec<(usize, usize, u16)> = Vec::new(); // (start, length, dict_addr)
        let mut word_start = 0;
        let mut in_word = false;
        
        let input_bytes: Vec<char> = input.chars().collect();
        for (i, &c) in input_bytes.iter().enumerate() {
            if c == ' ' {
                if in_word {
                    let word = &input[word_start..i];
                    let dict_entry = self.find_in_dictionary(word, entries_start, entry_length, num_entries);
                    tokens.push((word_start + 1, i - word_start, dict_entry));
                    in_word = false;
                }
            } else if separators.contains(&c) {
                if in_word {
                    let word = &input[word_start..i];
                    let dict_entry = self.find_in_dictionary(word, entries_start, entry_length, num_entries);
                    tokens.push((word_start + 1, i - word_start, dict_entry));
                    in_word = false;
                }
                // Separator is also a token
                let sep_str = c.to_string();
                let dict_entry = self.find_in_dictionary(&sep_str, entries_start, entry_length, num_entries);
                tokens.push((i + 1, 1, dict_entry));
            } else if !in_word {
                word_start = i;
                in_word = true;
            }
        }
        
        if in_word {
            let word = &input[word_start..];
            let dict_entry = self.find_in_dictionary(word, entries_start, entry_length, num_entries);
            tokens.push((word_start + 1, input.len() - word_start, dict_entry));
        }
        
        // Write parse buffer
        let num_tokens = tokens.len().min(max_tokens);
        self.memory.write_byte(parse_buffer_addr + 1, num_tokens as u8);
        
        for (i, &(start, length, dict_addr)) in tokens.iter().take(max_tokens).enumerate() {
            let token_addr = parse_buffer_addr + 2 + (i * 4);
            self.memory.write_word(token_addr, dict_addr);
            self.memory.write_byte(token_addr + 2, length as u8);
            self.memory.write_byte(token_addr + 3, start as u8);
        }
    }
    
    /// Find a word in the dictionary
    fn find_in_dictionary(&self, word: &str, entries_start: usize, entry_length: usize, num_entries: usize) -> u16 {
        let encoded = self.encode_zstring(word);
        
        // Binary search (dictionary is sorted)
        let mut low = 0;
        let mut high = num_entries;
        
        while low < high {
            let mid = (low + high) / 2;
            let entry_addr = entries_start + mid * entry_length;
            
            let entry_word = [
                self.memory.read_word(entry_addr),
                self.memory.read_word(entry_addr + 2),
            ];
            
            let cmp = if encoded[0] < entry_word[0] {
                std::cmp::Ordering::Less
            } else if encoded[0] > entry_word[0] {
                std::cmp::Ordering::Greater
            } else if encoded[1] < entry_word[1] {
                std::cmp::Ordering::Less
            } else if encoded[1] > entry_word[1] {
                std::cmp::Ordering::Greater
            } else {
                std::cmp::Ordering::Equal
            };
            
            match cmp {
                std::cmp::Ordering::Less => high = mid,
                std::cmp::Ordering::Greater => low = mid + 1,
                std::cmp::Ordering::Equal => return entry_addr as u16,
            }
        }
        
        0 // Not found
    }
    
    /// Encode a word as Z-characters
    fn encode_zstring(&self, word: &str) -> [u16; 2] {
        let mut zchars = [5u8; 6]; // Pad with 5s
        let alphabet = "abcdefghijklmnopqrstuvwxyz";
        
        for (i, c) in word.chars().take(6).enumerate() {
            if let Some(pos) = alphabet.find(c) {
                zchars[i] = (pos + 6) as u8;
            }
        }
        
        // Pack into 2 words
        let word1 = ((zchars[0] as u16) << 10) | ((zchars[1] as u16) << 5) | (zchars[2] as u16);
        let word2 = ((zchars[3] as u16) << 10) | ((zchars[4] as u16) << 5) | (zchars[5] as u16) | 0x8000;
        
        [word1, word2]
    }
    
    /// Get object attribute
    fn get_object_attr(&self, obj: u16, attr: u16) -> bool {
        if obj == 0 {
            return false;
        }
        
        let obj_table = self.memory.read_word(0x0A) as usize;
        let obj_addr = obj_table + 62 + ((obj as usize - 1) * 9);
        
        let byte_num = (attr / 8) as usize;
        let bit_num = 7 - (attr % 8);
        
        let byte = self.memory.read_byte(obj_addr + byte_num);
        (byte & (1 << bit_num)) != 0
    }
    
    /// Set object attribute
    fn set_object_attr(&mut self, obj: u16, attr: u16, value: bool) {
        if obj == 0 {
            return;
        }
        
        let obj_table = self.memory.read_word(0x0A) as usize;
        let obj_addr = obj_table + 62 + ((obj as usize - 1) * 9);
        
        let byte_num = (attr / 8) as usize;
        let bit_num = 7 - (attr % 8);
        
        let mut byte = self.memory.read_byte(obj_addr + byte_num);
        if value {
            byte |= 1 << bit_num;
        } else {
            byte &= !(1 << bit_num);
        }
        self.memory.write_byte(obj_addr + byte_num, byte);
    }
    
    /// Get object parent
    fn get_object_parent(&self, obj: u16) -> u16 {
        if obj == 0 {
            return 0;
        }
        let obj_table = self.memory.read_word(0x0A) as usize;
        let obj_addr = obj_table + 62 + ((obj as usize - 1) * 9);
        self.memory.read_byte(obj_addr + 4) as u16
    }
    
    /// Get object sibling
    fn get_object_sibling(&self, obj: u16) -> u16 {
        if obj == 0 {
            return 0;
        }
        let obj_table = self.memory.read_word(0x0A) as usize;
        let obj_addr = obj_table + 62 + ((obj as usize - 1) * 9);
        self.memory.read_byte(obj_addr + 5) as u16
    }
    
    /// Get object child
    fn get_object_child(&self, obj: u16) -> u16 {
        if obj == 0 {
            return 0;
        }
        let obj_table = self.memory.read_word(0x0A) as usize;
        let obj_addr = obj_table + 62 + ((obj as usize - 1) * 9);
        self.memory.read_byte(obj_addr + 6) as u16
    }
    
    /// Get object property address
    fn get_object_prop_addr(&self, obj: u16) -> usize {
        if obj == 0 {
            return 0;
        }
        let obj_table = self.memory.read_word(0x0A) as usize;
        let obj_addr = obj_table + 62 + ((obj as usize - 1) * 9);
        self.memory.read_word(obj_addr + 7) as usize
    }
    
    /// Get object short name
    fn get_object_name(&self, obj: u16) -> String {
        let prop_addr = self.get_object_prop_addr(obj);
        if prop_addr == 0 {
            return String::new();
        }
        let name_len = self.memory.read_byte(prop_addr) as usize;
        if name_len == 0 {
            return String::new();
        }
        self.decode_zstring(prop_addr + 1)
    }
    
    /// Get property value
    fn get_property(&self, obj: u16, prop: u16) -> u16 {
        let (_, prop_addr, prop_len) = self.find_property(obj, prop);
        
        if prop_addr == 0 {
            // Return default value
            let obj_table = self.memory.read_word(0x0A) as usize;
            let default_addr = obj_table + ((prop as usize - 1) * 2);
            return self.memory.read_word(default_addr);
        }
        
        match prop_len {
            1 => self.memory.read_byte(prop_addr) as u16,
            _ => self.memory.read_word(prop_addr),
        }
    }
    
    /// Set property value
    fn set_property(&mut self, obj: u16, prop: u16, value: u16) {
        let (_, prop_addr, prop_len) = self.find_property(obj, prop);
        
        if prop_addr == 0 {
            return;
        }
        
        match prop_len {
            1 => self.memory.write_byte(prop_addr, value as u8),
            _ => self.memory.write_word(prop_addr, value),
        }
    }
    
    /// Find a property, returns (size_byte_addr, data_addr, length)
    fn find_property(&self, obj: u16, prop: u16) -> (usize, usize, usize) {
        let prop_table = self.get_object_prop_addr(obj);
        if prop_table == 0 {
            return (0, 0, 0);
        }
        
        // Skip short name
        let name_len = self.memory.read_byte(prop_table) as usize;
        let mut addr = prop_table + 1 + (name_len * 2);
        
        loop {
            let size_byte = self.memory.read_byte(addr);
            if size_byte == 0 {
                break;
            }
            
            let prop_num = size_byte & 0x1F;
            let prop_len = (size_byte >> 5) as usize + 1;
            
            if prop_num as u16 == prop {
                return (addr, addr + 1, prop_len);
            }
            
            if (prop_num as u16) < prop {
                // Properties are in descending order
                break;
            }
            
            addr += 1 + prop_len;
        }
        
        (0, 0, 0)
    }
    
    /// Get next property number
    fn get_next_property(&self, obj: u16, prop: u16) -> u16 {
        let prop_table = self.get_object_prop_addr(obj);
        if prop_table == 0 {
            return 0;
        }
        
        // Skip short name
        let name_len = self.memory.read_byte(prop_table) as usize;
        let mut addr = prop_table + 1 + (name_len * 2);
        
        if prop == 0 {
            // Return first property
            let size_byte = self.memory.read_byte(addr);
            return (size_byte & 0x1F) as u16;
        }
        
        loop {
            let size_byte = self.memory.read_byte(addr);
            if size_byte == 0 {
                break;
            }
            
            let prop_num = size_byte & 0x1F;
            let prop_len = (size_byte >> 5) as usize + 1;
            
            if prop_num as u16 == prop {
                // Return next property
                addr += 1 + prop_len;
                let next_size = self.memory.read_byte(addr);
                return (next_size & 0x1F) as u16;
            }
            
            addr += 1 + prop_len;
        }
        
        0
    }
    
    /// Insert object into another object
    fn insert_object(&mut self, obj: u16, dest: u16) {
        self.remove_object(obj);
        
        if obj == 0 || dest == 0 {
            return;
        }
        
        let obj_table = self.memory.read_word(0x0A) as usize;
        let obj_addr = obj_table + 62 + ((obj as usize - 1) * 9);
        let dest_addr = obj_table + 62 + ((dest as usize - 1) * 9);
        
        // Get dest's current child
        let dest_child = self.memory.read_byte(dest_addr + 6);
        
        // Set obj's parent to dest
        self.memory.write_byte(obj_addr + 4, dest as u8);
        
        // Set obj's sibling to dest's old child
        self.memory.write_byte(obj_addr + 5, dest_child);
        
        // Set dest's child to obj
        self.memory.write_byte(dest_addr + 6, obj as u8);
    }
    
    /// Remove object from its parent
    fn remove_object(&mut self, obj: u16) {
        if obj == 0 {
            return;
        }
        
        let parent = self.get_object_parent(obj);
        if parent == 0 {
            return;
        }
        
        let obj_table = self.memory.read_word(0x0A) as usize;
        let obj_addr = obj_table + 62 + ((obj as usize - 1) * 9);
        let parent_addr = obj_table + 62 + ((parent as usize - 1) * 9);
        
        let obj_sibling = self.memory.read_byte(obj_addr + 5);
        let parent_child = self.memory.read_byte(parent_addr + 6);
        
        if parent_child == obj as u8 {
            // obj is first child
            self.memory.write_byte(parent_addr + 6, obj_sibling);
        } else {
            // Find previous sibling
            let mut prev = parent_child;
            loop {
                if prev == 0 {
                    break;
                }
                let prev_addr = obj_table + 62 + ((prev as usize - 1) * 9);
                let next = self.memory.read_byte(prev_addr + 5);
                if next == obj as u8 {
                    self.memory.write_byte(prev_addr + 5, obj_sibling);
                    break;
                }
                prev = next;
            }
        }
        
        // Clear obj's parent and sibling
        self.memory.write_byte(obj_addr + 4, 0);
        self.memory.write_byte(obj_addr + 5, 0);
    }
    
    /// Generate random number
    fn random(&mut self, range: i16) -> u16 {
        use std::time::{SystemTime, UNIX_EPOCH};
        
        if range <= 0 {
            // Seed the random number generator
            0
        } else {
            // Generate random number 1..range
            let seed = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos() as u32;
            (seed % range as u32 + 1) as u16
        }
    }
}
