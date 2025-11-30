//! Z-Machine interpreter module
//!
//! This module implements a Z-machine virtual machine interpreter for
//! running Infocom games like Zork I.

mod memory;
mod opcodes;
mod text;
mod objects;
mod io;

pub use self::memory::Memory;
pub use self::text::TextEngine;
pub use self::objects::ObjectTable;
pub use self::io::IOSystem;

/// Error type for Z-machine operations
#[derive(Debug)]
pub struct ZMachineError {
    pub message: String,
}

impl std::fmt::Display for ZMachineError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for ZMachineError {}

impl ZMachineError {
    pub fn new(message: impl Into<String>) -> Self {
        ZMachineError {
            message: message.into(),
        }
    }
}

/// The main Z-machine interpreter
pub struct ZMachine {
    memory: Memory,
    text_engine: TextEngine,
    io: IOSystem,
    
    // Registers
    pc: usize,           // Program counter
    stack: Vec<u16>,     // Main stack
    call_stack: Vec<CallFrame>,  // Call stack frames
    
    // State
    running: bool,
    version: u8,
}

/// A call frame for routine calls
#[derive(Clone)]
struct CallFrame {
    return_pc: usize,
    local_vars: Vec<u16>,
    stack_start: usize,
    store_var: Option<u8>,
    arg_count: u8,
}

impl ZMachine {
    /// Create a new Z-machine with the given story data
    pub fn new(story_data: Vec<u8>) -> Result<Self, ZMachineError> {
        if story_data.len() < 64 {
            return Err(ZMachineError::new("Story file too small"));
        }
        
        let version = story_data[0];
        if version < 1 || version > 3 {
            return Err(ZMachineError::new(format!(
                "Unsupported Z-machine version: {} (only versions 1-3 supported)",
                version
            )));
        }
        
        let memory = Memory::new(story_data)?;
        let text_engine = TextEngine::new(version);
        let io = IOSystem::new();
        
        // Initial PC comes from the header at offset 0x06-0x07
        let initial_pc = memory.read_word(0x06) as usize;
        
        Ok(ZMachine {
            memory,
            text_engine,
            io,
            pc: initial_pc,
            stack: Vec::new(),
            call_stack: Vec::new(),
            running: true,
            version,
        })
    }
    
    /// Run the Z-machine until it halts
    pub fn run(&mut self) -> Result<(), ZMachineError> {
        while self.running {
            self.execute_instruction()?;
        }
        Ok(())
    }
    
    /// Execute a single instruction
    fn execute_instruction(&mut self) -> Result<(), ZMachineError> {
        let opcode_byte = self.read_byte();
        
        // Decode instruction form
        let (opcode, operands) = self.decode_instruction(opcode_byte)?;
        
        // Execute the opcode
        self.execute_opcode(opcode, operands)?;
        
        Ok(())
    }
    
    /// Decode an instruction and its operands
    fn decode_instruction(&mut self, opcode_byte: u8) -> Result<(u8, Vec<Operand>), ZMachineError> {
        if opcode_byte == 0xBE {
            // Extended opcode (version 5+, not needed for v3)
            return Err(ZMachineError::new("Extended opcodes not supported in v3"));
        }
        
        // Determine instruction form
        let form = if opcode_byte & 0x80 == 0 {
            // Long form: 2OP
            InstructionForm::Long
        } else if opcode_byte & 0x40 == 0 {
            // Short form: 1OP or 0OP
            InstructionForm::Short
        } else {
            // Variable form: 2OP or VAR
            InstructionForm::Variable
        };
        
        let (opcode, operands) = match form {
            InstructionForm::Long => {
                // Long form: bits 6-5 give operand types, bits 4-0 give opcode
                let opcode = opcode_byte & 0x1F;
                let op1_type = if opcode_byte & 0x40 == 0 { OperandType::Small } else { OperandType::Variable };
                let op2_type = if opcode_byte & 0x20 == 0 { OperandType::Small } else { OperandType::Variable };
                
                let op1 = self.read_operand(op1_type)?;
                let op2 = self.read_operand(op2_type)?;
                
                (opcode, vec![op1, op2])
            }
            InstructionForm::Short => {
                // Short form: bits 5-4 give operand type, bits 3-0 give opcode
                let op_type_bits = (opcode_byte >> 4) & 0x03;
                let opcode = opcode_byte & 0x0F;
                
                let operands = if op_type_bits == 0x03 {
                    // 0OP: opcode is 0OP + base 176
                    vec![]
                } else {
                    // 1OP: opcode is 1OP + base 128
                    let op_type = match op_type_bits {
                        0 => OperandType::Large,
                        1 => OperandType::Small,
                        2 => OperandType::Variable,
                        _ => unreachable!(),
                    };
                    vec![self.read_operand(op_type)?]
                };
                
                let full_opcode = if op_type_bits == 0x03 {
                    opcode + 176  // 0OP base
                } else {
                    opcode + 128  // 1OP base
                };
                
                (full_opcode, operands)
            }
            InstructionForm::Variable => {
                let is_2op = opcode_byte & 0x20 == 0;
                let opcode = if is_2op {
                    opcode_byte & 0x1F  // 2OP
                } else {
                    (opcode_byte & 0x1F) + 224  // VAR opcode base
                };
                
                // Read operand types from following byte(s)
                let types_byte = self.read_byte();
                let operands = self.read_operands_from_types(types_byte)?;
                
                (opcode, operands)
            }
        };
        
        Ok((opcode, operands))
    }
    
    /// Read operands based on a types byte
    fn read_operands_from_types(&mut self, types_byte: u8) -> Result<Vec<Operand>, ZMachineError> {
        let mut operands = Vec::new();
        
        for i in 0..4 {
            let shift = 6 - (i * 2);
            let op_type_bits = (types_byte >> shift) & 0x03;
            
            match op_type_bits {
                0b11 => break,  // Omitted
                0b00 => operands.push(self.read_operand(OperandType::Large)?),
                0b01 => operands.push(self.read_operand(OperandType::Small)?),
                0b10 => operands.push(self.read_operand(OperandType::Variable)?),
                _ => unreachable!(),
            }
        }
        
        Ok(operands)
    }
    
    /// Read an operand of the given type
    fn read_operand(&mut self, op_type: OperandType) -> Result<Operand, ZMachineError> {
        match op_type {
            OperandType::Large => {
                let value = self.read_word();
                Ok(Operand::Large(value))
            }
            OperandType::Small => {
                let value = self.read_byte();
                Ok(Operand::Small(value))
            }
            OperandType::Variable => {
                let var_num = self.read_byte();
                let value = self.read_variable(var_num)?;
                Ok(Operand::Variable(value))
            }
        }
    }
    
    /// Read a byte from memory at PC and advance PC
    fn read_byte(&mut self) -> u8 {
        let byte = self.memory.read_byte(self.pc);
        self.pc += 1;
        byte
    }
    
    /// Read a word from memory at PC and advance PC
    fn read_word(&mut self) -> u16 {
        let word = self.memory.read_word(self.pc);
        self.pc += 2;
        word
    }
    
    /// Read a variable value
    fn read_variable(&mut self, var_num: u8) -> Result<u16, ZMachineError> {
        if var_num == 0 {
            // Stack
            self.stack.pop().ok_or_else(|| ZMachineError::new("Stack underflow"))
        } else if var_num <= 15 {
            // Local variable
            let frame = self.call_stack.last()
                .ok_or_else(|| ZMachineError::new("No call frame for local variable"))?;
            let local_index = (var_num - 1) as usize;
            if local_index >= frame.local_vars.len() {
                return Err(ZMachineError::new(format!("Invalid local variable: {}", var_num)));
            }
            Ok(frame.local_vars[local_index])
        } else {
            // Global variable
            let global_index = (var_num - 16) as usize;
            let globals_addr = self.memory.read_word(0x0C) as usize;
            let addr = globals_addr + global_index * 2;
            Ok(self.memory.read_word(addr))
        }
    }
    
    /// Write a variable value
    fn write_variable(&mut self, var_num: u8, value: u16) -> Result<(), ZMachineError> {
        if var_num == 0 {
            // Stack
            self.stack.push(value);
        } else if var_num <= 15 {
            // Local variable
            let frame = self.call_stack.last_mut()
                .ok_or_else(|| ZMachineError::new("No call frame for local variable"))?;
            let local_index = (var_num - 1) as usize;
            if local_index >= frame.local_vars.len() {
                return Err(ZMachineError::new(format!("Invalid local variable: {}", var_num)));
            }
            frame.local_vars[local_index] = value;
        } else {
            // Global variable
            let global_index = (var_num - 16) as usize;
            let globals_addr = self.memory.read_word(0x0C) as usize;
            let addr = globals_addr + global_index * 2;
            self.memory.write_word(addr, value);
        }
        Ok(())
    }
    
    /// Store a result in a variable
    fn store_result(&mut self, value: u16) -> Result<(), ZMachineError> {
        let var_num = self.read_byte();
        self.write_variable(var_num, value)
    }
    
    /// Handle a branch instruction
    fn branch(&mut self, condition: bool) -> Result<(), ZMachineError> {
        let branch_byte = self.read_byte();
        let branch_on_true = branch_byte & 0x80 != 0;
        
        let offset = if branch_byte & 0x40 != 0 {
            // Single byte offset (6 bits)
            (branch_byte & 0x3F) as i16
        } else {
            // Two byte offset (14 bits, signed)
            let second_byte = self.read_byte();
            let offset_14 = ((branch_byte & 0x3F) as u16) << 8 | (second_byte as u16);
            // Sign extend 14-bit to 16-bit
            if offset_14 & 0x2000 != 0 {
                (offset_14 | 0xC000) as i16
            } else {
                offset_14 as i16
            }
        };
        
        let should_branch = condition == branch_on_true;
        
        if should_branch {
            match offset {
                0 => self.return_from_routine(0)?,  // Return false
                1 => self.return_from_routine(1)?,  // Return true
                _ => {
                    // Jump to offset - 2 from current position
                    self.pc = ((self.pc as i32) + (offset as i32) - 2) as usize;
                }
            }
        }
        
        Ok(())
    }
    
    /// Call a routine
    fn call_routine(&mut self, packed_addr: u16, args: &[u16], store_var: Option<u8>) -> Result<(), ZMachineError> {
        if packed_addr == 0 {
            // Calling address 0 returns 0/false
            if let Some(var) = store_var {
                self.write_variable(var, 0)?;
            }
            return Ok(());
        }
        
        // Unpack routine address (for v3, multiply by 2)
        let routine_addr = (packed_addr as usize) * 2;
        
        // Read number of local variables
        let num_locals = self.memory.read_byte(routine_addr) as usize;
        if num_locals > 15 {
            return Err(ZMachineError::new(format!("Too many local variables: {}", num_locals)));
        }
        
        // Initialize local variables
        let mut local_vars = Vec::with_capacity(num_locals);
        for i in 0..num_locals {
            // In v3, locals are initialized from the routine header
            let init_value = self.memory.read_word(routine_addr + 1 + i * 2);
            local_vars.push(init_value);
        }
        
        // Override with arguments
        let arg_count = args.len().min(num_locals);
        for (i, &arg) in args.iter().take(arg_count).enumerate() {
            local_vars[i] = arg;
        }
        
        // Push call frame
        let frame = CallFrame {
            return_pc: self.pc,
            local_vars,
            stack_start: self.stack.len(),
            store_var,
            arg_count: arg_count as u8,
        };
        self.call_stack.push(frame);
        
        // Set PC to first instruction after locals
        self.pc = routine_addr + 1 + num_locals * 2;
        
        Ok(())
    }
    
    /// Return from current routine
    fn return_from_routine(&mut self, value: u16) -> Result<(), ZMachineError> {
        let frame = self.call_stack.pop()
            .ok_or_else(|| ZMachineError::new("Return from empty call stack"))?;
        
        // Restore stack
        self.stack.truncate(frame.stack_start);
        
        // Restore PC
        self.pc = frame.return_pc;
        
        // Store return value if needed
        if let Some(var) = frame.store_var {
            self.write_variable(var, value)?;
        }
        
        Ok(())
    }
    
    /// Get the value of an operand
    fn operand_value(&self, operand: &Operand) -> u16 {
        match operand {
            Operand::Large(v) => *v,
            Operand::Small(v) => *v as u16,
            Operand::Variable(v) => *v,
        }
    }
    
    /// Print a Z-string at the given address
    fn print_zstring(&mut self, addr: usize) -> usize {
        let (text, bytes_read) = self.text_engine.decode(&self.memory, addr);
        self.io.print(&text);
        bytes_read
    }
    
    /// Read a line of input from the user
    fn read_line(&mut self, text_buffer_addr: usize, parse_buffer_addr: usize) -> Result<(), ZMachineError> {
        // Print any pending output
        self.io.flush();
        
        // Read input line
        let input = self.io.read_line()?;
        let input = input.to_lowercase();
        let input_bytes = input.as_bytes();
        
        // Write to text buffer
        // First byte is max length, we write starting at byte 1
        let max_len = self.memory.read_byte(text_buffer_addr) as usize;
        let write_len = input_bytes.len().min(max_len);
        
        for (i, &byte) in input_bytes.iter().take(write_len).enumerate() {
            self.memory.write_byte(text_buffer_addr + 1 + i, byte);
        }
        
        // Null terminate
        if write_len < max_len {
            self.memory.write_byte(text_buffer_addr + 1 + write_len, 0);
        }
        
        // Tokenize input
        self.tokenize(&input, parse_buffer_addr)?;
        
        Ok(())
    }
    
    /// Tokenize input into the parse buffer
    fn tokenize(&mut self, input: &str, parse_buffer_addr: usize) -> Result<(), ZMachineError> {
        let dictionary_addr = self.memory.read_word(0x08) as usize;
        
        // Read dictionary header
        let num_separators = self.memory.read_byte(dictionary_addr) as usize;
        let mut separators = Vec::new();
        for i in 0..num_separators {
            separators.push(self.memory.read_byte(dictionary_addr + 1 + i) as char);
        }
        
        let entry_length = self.memory.read_byte(dictionary_addr + 1 + num_separators) as usize;
        let num_entries = self.memory.read_word(dictionary_addr + 2 + num_separators) as usize;
        let entries_start = dictionary_addr + 4 + num_separators;
        
        // Parse tokens
        let max_tokens = self.memory.read_byte(parse_buffer_addr) as usize;
        let mut token_count = 0;
        let mut pos = 0;
        
        let input_bytes = input.as_bytes();
        
        while pos < input.len() && token_count < max_tokens {
            // Skip whitespace
            while pos < input.len() && input_bytes[pos] == b' ' {
                pos += 1;
            }
            
            if pos >= input.len() {
                break;
            }
            
            let token_start = pos;
            let mut is_separator = false;
            
            // Check if current char is a separator
            let current_char = input_bytes[pos] as char;
            for &sep in &separators {
                if current_char == sep {
                    is_separator = true;
                    pos += 1;
                    break;
                }
            }
            
            if !is_separator {
                // Read word until separator or whitespace
                while pos < input.len() {
                    let c = input_bytes[pos] as char;
                    if c == ' ' || separators.contains(&c) {
                        break;
                    }
                    pos += 1;
                }
            }
            
            let token = &input[token_start..pos];
            
            // Look up token in dictionary
            let dict_addr = self.lookup_word(token, entries_start, entry_length, num_entries);
            
            // Write to parse buffer
            let token_offset = parse_buffer_addr + 2 + token_count * 4;
            self.memory.write_word(token_offset, dict_addr as u16);
            self.memory.write_byte(token_offset + 2, (pos - token_start) as u8);
            self.memory.write_byte(token_offset + 3, (token_start + 1) as u8);  // 1-indexed
            
            token_count += 1;
        }
        
        // Write token count
        self.memory.write_byte(parse_buffer_addr + 1, token_count as u8);
        
        Ok(())
    }
    
    /// Look up a word in the dictionary
    fn lookup_word(&self, word: &str, entries_start: usize, entry_length: usize, num_entries: usize) -> usize {
        // Encode the word as a Z-string
        let encoded = self.text_engine.encode_word(word);
        
        // Binary search (dictionary is sorted)
        let mut low = 0;
        let mut high = num_entries;
        
        while low < high {
            let mid = (low + high) / 2;
            let entry_addr = entries_start + mid * entry_length;
            
            // Compare encoded word with dictionary entry
            let mut cmp = std::cmp::Ordering::Equal;
            for i in 0..encoded.len() {
                let dict_word = self.memory.read_word(entry_addr + i * 2);
                cmp = encoded[i].cmp(&dict_word);
                if cmp != std::cmp::Ordering::Equal {
                    break;
                }
            }
            
            match cmp {
                std::cmp::Ordering::Equal => return entry_addr,
                std::cmp::Ordering::Less => high = mid,
                std::cmp::Ordering::Greater => low = mid + 1,
            }
        }
        
        0  // Not found
    }
    
    /// Execute an opcode
    fn execute_opcode(&mut self, opcode: u8, operands: Vec<Operand>) -> Result<(), ZMachineError> {
        let ops: Vec<u16> = operands.iter().map(|o| self.operand_value(o)).collect();
        
        match opcode {
            // 2OP opcodes (0x01-0x1F)
            0x01 => { // je
                let a = ops.first().copied().unwrap_or(0);
                let matches = ops.iter().skip(1).any(|&x| x == a);
                self.branch(matches)?;
            }
            0x02 => { // jl
                let a = ops.first().copied().unwrap_or(0) as i16;
                let b = ops.get(1).copied().unwrap_or(0) as i16;
                self.branch(a < b)?;
            }
            0x03 => { // jg
                let a = ops.first().copied().unwrap_or(0) as i16;
                let b = ops.get(1).copied().unwrap_or(0) as i16;
                self.branch(a > b)?;
            }
            0x04 => { // dec_chk
                let var = ops.first().copied().unwrap_or(0) as u8;
                let mut value = self.read_variable(var)? as i16;
                value -= 1;
                self.write_variable(var, value as u16)?;
                let check = ops.get(1).copied().unwrap_or(0) as i16;
                self.branch(value < check)?;
            }
            0x05 => { // inc_chk
                let var = ops.first().copied().unwrap_or(0) as u8;
                let mut value = self.read_variable(var)? as i16;
                value += 1;
                self.write_variable(var, value as u16)?;
                let check = ops.get(1).copied().unwrap_or(0) as i16;
                self.branch(value > check)?;
            }
            0x06 => { // jin
                let obj1 = ops.first().copied().unwrap_or(0) as u8;
                let obj2 = ops.get(1).copied().unwrap_or(0) as u8;
                let parent = self.get_object_parent(obj1);
                self.branch(parent == obj2)?;
            }
            0x07 => { // test
                let bitmap = ops.first().copied().unwrap_or(0);
                let flags = ops.get(1).copied().unwrap_or(0);
                self.branch((bitmap & flags) == flags)?;
            }
            0x08 => { // or
                let a = ops.first().copied().unwrap_or(0);
                let b = ops.get(1).copied().unwrap_or(0);
                self.store_result(a | b)?;
            }
            0x09 => { // and
                let a = ops.first().copied().unwrap_or(0);
                let b = ops.get(1).copied().unwrap_or(0);
                self.store_result(a & b)?;
            }
            0x0A => { // test_attr
                let obj = ops.first().copied().unwrap_or(0) as u8;
                let attr = ops.get(1).copied().unwrap_or(0) as u8;
                let has_attr = self.test_object_attr(obj, attr);
                self.branch(has_attr)?;
            }
            0x0B => { // set_attr
                let obj = ops.first().copied().unwrap_or(0) as u8;
                let attr = ops.get(1).copied().unwrap_or(0) as u8;
                self.set_object_attr(obj, attr, true);
            }
            0x0C => { // clear_attr
                let obj = ops.first().copied().unwrap_or(0) as u8;
                let attr = ops.get(1).copied().unwrap_or(0) as u8;
                self.set_object_attr(obj, attr, false);
            }
            0x0D => { // store
                let var = ops.first().copied().unwrap_or(0) as u8;
                let value = ops.get(1).copied().unwrap_or(0);
                self.write_variable(var, value)?;
            }
            0x0E => { // insert_obj
                let obj = ops.first().copied().unwrap_or(0) as u8;
                let dest = ops.get(1).copied().unwrap_or(0) as u8;
                self.insert_object(obj, dest);
            }
            0x0F => { // loadw
                let array = ops.first().copied().unwrap_or(0) as usize;
                let index = ops.get(1).copied().unwrap_or(0) as usize;
                let value = self.memory.read_word(array + index * 2);
                self.store_result(value)?;
            }
            0x10 => { // loadb
                let array = ops.first().copied().unwrap_or(0) as usize;
                let index = ops.get(1).copied().unwrap_or(0) as usize;
                let value = self.memory.read_byte(array + index) as u16;
                self.store_result(value)?;
            }
            0x11 => { // get_prop
                let obj = ops.first().copied().unwrap_or(0) as u8;
                let prop = ops.get(1).copied().unwrap_or(0) as u8;
                let value = self.get_object_property(obj, prop);
                self.store_result(value)?;
            }
            0x12 => { // get_prop_addr
                let obj = ops.first().copied().unwrap_or(0) as u8;
                let prop = ops.get(1).copied().unwrap_or(0) as u8;
                let addr = self.get_object_property_addr(obj, prop);
                self.store_result(addr as u16)?;
            }
            0x13 => { // get_next_prop
                let obj = ops.first().copied().unwrap_or(0) as u8;
                let prop = ops.get(1).copied().unwrap_or(0) as u8;
                let next = self.get_object_next_property(obj, prop);
                self.store_result(next as u16)?;
            }
            0x14 => { // add
                let a = ops.first().copied().unwrap_or(0) as i16;
                let b = ops.get(1).copied().unwrap_or(0) as i16;
                self.store_result(a.wrapping_add(b) as u16)?;
            }
            0x15 => { // sub
                let a = ops.first().copied().unwrap_or(0) as i16;
                let b = ops.get(1).copied().unwrap_or(0) as i16;
                self.store_result(a.wrapping_sub(b) as u16)?;
            }
            0x16 => { // mul
                let a = ops.first().copied().unwrap_or(0) as i16;
                let b = ops.get(1).copied().unwrap_or(0) as i16;
                self.store_result(a.wrapping_mul(b) as u16)?;
            }
            0x17 => { // div
                let a = ops.first().copied().unwrap_or(0) as i16;
                let b = ops.get(1).copied().unwrap_or(0) as i16;
                if b == 0 {
                    return Err(ZMachineError::new("Division by zero"));
                }
                self.store_result((a / b) as u16)?;
            }
            0x18 => { // mod
                let a = ops.first().copied().unwrap_or(0) as i16;
                let b = ops.get(1).copied().unwrap_or(0) as i16;
                if b == 0 {
                    return Err(ZMachineError::new("Division by zero"));
                }
                self.store_result((a % b) as u16)?;
            }
            
            // 1OP opcodes (0x80-0x8F)
            0x80 => { // jz
                let a = ops.first().copied().unwrap_or(0);
                self.branch(a == 0)?;
            }
            0x81 => { // get_sibling
                let obj = ops.first().copied().unwrap_or(0) as u8;
                let sibling = self.get_object_sibling(obj);
                self.store_result(sibling as u16)?;
                self.branch(sibling != 0)?;
            }
            0x82 => { // get_child
                let obj = ops.first().copied().unwrap_or(0) as u8;
                let child = self.get_object_child(obj);
                self.store_result(child as u16)?;
                self.branch(child != 0)?;
            }
            0x83 => { // get_parent
                let obj = ops.first().copied().unwrap_or(0) as u8;
                let parent = self.get_object_parent(obj);
                self.store_result(parent as u16)?;
            }
            0x84 => { // get_prop_len
                let prop_addr = ops.first().copied().unwrap_or(0) as usize;
                let len = if prop_addr == 0 {
                    0
                } else {
                    let size_byte = self.memory.read_byte(prop_addr - 1);
                    (size_byte / 32) + 1
                };
                self.store_result(len as u16)?;
            }
            0x85 => { // inc
                let var = ops.first().copied().unwrap_or(0) as u8;
                let value = self.read_variable(var)? as i16;
                self.write_variable(var, value.wrapping_add(1) as u16)?;
            }
            0x86 => { // dec
                let var = ops.first().copied().unwrap_or(0) as u8;
                let value = self.read_variable(var)? as i16;
                self.write_variable(var, value.wrapping_sub(1) as u16)?;
            }
            0x87 => { // print_addr
                let addr = ops.first().copied().unwrap_or(0) as usize;
                self.print_zstring(addr);
            }
            0x89 => { // remove_obj
                let obj = ops.first().copied().unwrap_or(0) as u8;
                self.remove_object(obj);
            }
            0x8A => { // print_obj
                let obj = ops.first().copied().unwrap_or(0) as u8;
                let name = self.get_object_short_name(obj);
                self.io.print(&name);
            }
            0x8B => { // ret
                let value = ops.first().copied().unwrap_or(0);
                self.return_from_routine(value)?;
            }
            0x8C => { // jump
                let offset = ops.first().copied().unwrap_or(0) as i16;
                self.pc = ((self.pc as i32) + (offset as i32) - 2) as usize;
            }
            0x8D => { // print_paddr
                let packed_addr = ops.first().copied().unwrap_or(0) as usize;
                let addr = packed_addr * 2;  // v3 packing
                self.print_zstring(addr);
            }
            0x8E => { // load
                let var = ops.first().copied().unwrap_or(0) as u8;
                let value = self.read_variable(var)?;
                self.store_result(value)?;
            }
            0x8F => { // not (v1-4)
                let value = ops.first().copied().unwrap_or(0);
                self.store_result(!value)?;
            }
            
            // 0OP opcodes (0xB0-0xBF)
            0xB0 => { // rtrue
                self.return_from_routine(1)?;
            }
            0xB1 => { // rfalse
                self.return_from_routine(0)?;
            }
            0xB2 => { // print
                let bytes_read = self.print_zstring(self.pc);
                self.pc += bytes_read;
            }
            0xB3 => { // print_ret
                let bytes_read = self.print_zstring(self.pc);
                self.pc += bytes_read;
                self.io.print("\n");
                self.return_from_routine(1)?;
            }
            0xB4 => { // nop
                // Do nothing
            }
            0xB5 => { // save
                // Simple save - just branch on failure for now
                self.io.print("[Save not implemented]\n");
                self.branch(false)?;
            }
            0xB6 => { // restore
                // Simple restore - just branch on failure for now
                self.io.print("[Restore not implemented]\n");
                self.branch(false)?;
            }
            0xB7 => { // restart
                // Restart would reload the story file
                return Err(ZMachineError::new("Restart not implemented"));
            }
            0xB8 => { // ret_popped
                let value = self.stack.pop()
                    .ok_or_else(|| ZMachineError::new("Stack underflow"))?;
                self.return_from_routine(value)?;
            }
            0xB9 => { // pop
                self.stack.pop();
            }
            0xBA => { // quit
                self.running = false;
            }
            0xBB => { // new_line
                self.io.print("\n");
            }
            0xBC => { // show_status
                self.show_status();
            }
            0xBD => { // verify
                // For simplicity, always verify as true
                self.branch(true)?;
            }
            
            // VAR opcodes (0xE0-0xFF)
            0xE0 => { // call
                let routine = ops.first().copied().unwrap_or(0);
                let args: Vec<u16> = ops.iter().skip(1).copied().collect();
                let store_var = self.read_byte();
                self.call_routine(routine, &args, Some(store_var))?;
            }
            0xE1 => { // storew
                let array = ops.first().copied().unwrap_or(0) as usize;
                let index = ops.get(1).copied().unwrap_or(0) as usize;
                let value = ops.get(2).copied().unwrap_or(0);
                self.memory.write_word(array + index * 2, value);
            }
            0xE2 => { // storeb
                let array = ops.first().copied().unwrap_or(0) as usize;
                let index = ops.get(1).copied().unwrap_or(0) as usize;
                let value = ops.get(2).copied().unwrap_or(0) as u8;
                self.memory.write_byte(array + index, value);
            }
            0xE3 => { // put_prop
                let obj = ops.first().copied().unwrap_or(0) as u8;
                let prop = ops.get(1).copied().unwrap_or(0) as u8;
                let value = ops.get(2).copied().unwrap_or(0);
                self.put_object_property(obj, prop, value);
            }
            0xE4 => { // sread / aread
                let text_buffer = ops.first().copied().unwrap_or(0) as usize;
                let parse_buffer = ops.get(1).copied().unwrap_or(0) as usize;
                self.show_status();
                self.read_line(text_buffer, parse_buffer)?;
            }
            0xE5 => { // print_char
                let ch = ops.first().copied().unwrap_or(0) as u8 as char;
                self.io.print(&ch.to_string());
            }
            0xE6 => { // print_num
                let num = ops.first().copied().unwrap_or(0) as i16;
                self.io.print(&num.to_string());
            }
            0xE7 => { // random
                let range = ops.first().copied().unwrap_or(0) as i16;
                let result = if range <= 0 {
                    // Seed random number generator
                    0
                } else {
                    // Generate random number 1..range
                    (rand_simple() % (range as u32)) as u16 + 1
                };
                self.store_result(result)?;
            }
            0xE8 => { // push
                let value = ops.first().copied().unwrap_or(0);
                self.stack.push(value);
            }
            0xE9 => { // pull
                let var = ops.first().copied().unwrap_or(0) as u8;
                let value = self.stack.pop()
                    .ok_or_else(|| ZMachineError::new("Stack underflow"))?;
                self.write_variable(var, value)?;
            }
            0xEA => { // split_window
                // Not implemented for v3 text mode
            }
            0xEB => { // set_window
                // Not implemented for v3 text mode
            }
            0xED => { // erase_window
                // Not implemented for v3 text mode
            }
            0xF3 => { // output_stream
                // Simplified - just ignore stream switching
            }
            0xF4 => { // input_stream
                // Simplified - just ignore stream switching
            }
            0xF5 => { // sound_effect
                // Not implemented
            }
            
            _ => {
                return Err(ZMachineError::new(format!(
                    "Unknown opcode: 0x{:02X} at PC 0x{:04X}",
                    opcode, self.pc - 1
                )));
            }
        }
        
        Ok(())
    }
    
    // Object table operations
    fn get_object_addr(&self, obj: u8) -> usize {
        if obj == 0 {
            return 0;
        }
        let obj_table = self.memory.read_word(0x0A) as usize;
        // Skip 31 default properties (2 bytes each) = 62 bytes
        // Each object is 9 bytes in v3
        obj_table + 62 + ((obj as usize - 1) * 9)
    }
    
    fn get_object_parent(&self, obj: u8) -> u8 {
        if obj == 0 { return 0; }
        let addr = self.get_object_addr(obj);
        self.memory.read_byte(addr + 4)
    }
    
    fn get_object_sibling(&self, obj: u8) -> u8 {
        if obj == 0 { return 0; }
        let addr = self.get_object_addr(obj);
        self.memory.read_byte(addr + 5)
    }
    
    fn get_object_child(&self, obj: u8) -> u8 {
        if obj == 0 { return 0; }
        let addr = self.get_object_addr(obj);
        self.memory.read_byte(addr + 6)
    }
    
    fn set_object_parent(&mut self, obj: u8, parent: u8) {
        if obj == 0 { return; }
        let addr = self.get_object_addr(obj);
        self.memory.write_byte(addr + 4, parent);
    }
    
    fn set_object_sibling(&mut self, obj: u8, sibling: u8) {
        if obj == 0 { return; }
        let addr = self.get_object_addr(obj);
        self.memory.write_byte(addr + 5, sibling);
    }
    
    fn set_object_child(&mut self, obj: u8, child: u8) {
        if obj == 0 { return; }
        let addr = self.get_object_addr(obj);
        self.memory.write_byte(addr + 6, child);
    }
    
    fn test_object_attr(&self, obj: u8, attr: u8) -> bool {
        if obj == 0 || attr >= 32 { return false; }
        let addr = self.get_object_addr(obj);
        let byte_index = (attr / 8) as usize;
        let bit_index = 7 - (attr % 8);
        let byte = self.memory.read_byte(addr + byte_index);
        (byte >> bit_index) & 1 == 1
    }
    
    fn set_object_attr(&mut self, obj: u8, attr: u8, value: bool) {
        if obj == 0 || attr >= 32 { return; }
        let addr = self.get_object_addr(obj);
        let byte_index = (attr / 8) as usize;
        let bit_index = 7 - (attr % 8);
        let mut byte = self.memory.read_byte(addr + byte_index);
        if value {
            byte |= 1 << bit_index;
        } else {
            byte &= !(1 << bit_index);
        }
        self.memory.write_byte(addr + byte_index, byte);
    }
    
    fn get_object_property_table_addr(&self, obj: u8) -> usize {
        if obj == 0 { return 0; }
        let addr = self.get_object_addr(obj);
        self.memory.read_word(addr + 7) as usize
    }
    
    fn get_object_short_name(&self, obj: u8) -> String {
        if obj == 0 { return String::new(); }
        let prop_table = self.get_object_property_table_addr(obj);
        let name_len = self.memory.read_byte(prop_table) as usize;
        if name_len == 0 {
            return String::new();
        }
        let (text, _) = self.text_engine.decode(&self.memory, prop_table + 1);
        text
    }
    
    fn get_object_property(&self, obj: u8, prop: u8) -> u16 {
        if obj == 0 { return 0; }
        let prop_addr = self.get_object_property_addr(obj, prop);
        if prop_addr == 0 {
            // Return default property
            let obj_table = self.memory.read_word(0x0A) as usize;
            let default_addr = obj_table + ((prop as usize - 1) * 2);
            return self.memory.read_word(default_addr);
        }
        
        let size_byte = self.memory.read_byte(prop_addr - 1);
        let prop_len = (size_byte / 32) + 1;
        
        if prop_len == 1 {
            self.memory.read_byte(prop_addr) as u16
        } else {
            self.memory.read_word(prop_addr)
        }
    }
    
    fn get_object_property_addr(&self, obj: u8, prop: u8) -> usize {
        if obj == 0 { return 0; }
        let prop_table = self.get_object_property_table_addr(obj);
        let name_len = self.memory.read_byte(prop_table) as usize;
        let mut addr = prop_table + 1 + name_len * 2;
        
        loop {
            let size_byte = self.memory.read_byte(addr);
            if size_byte == 0 {
                return 0;  // Property not found
            }
            let prop_num = size_byte & 0x1F;
            let prop_len = (size_byte / 32) + 1;
            
            if prop_num == prop {
                return addr + 1;
            }
            if prop_num < prop {
                return 0;  // Properties are in descending order
            }
            addr += 1 + prop_len as usize;
        }
    }
    
    fn get_object_next_property(&self, obj: u8, prop: u8) -> u8 {
        if obj == 0 { return 0; }
        let prop_table = self.get_object_property_table_addr(obj);
        let name_len = self.memory.read_byte(prop_table) as usize;
        let mut addr = prop_table + 1 + name_len * 2;
        
        if prop == 0 {
            // Return first property
            let size_byte = self.memory.read_byte(addr);
            return size_byte & 0x1F;
        }
        
        // Find the given property first
        loop {
            let size_byte = self.memory.read_byte(addr);
            if size_byte == 0 {
                return 0;
            }
            let prop_num = size_byte & 0x1F;
            let prop_len = (size_byte / 32) + 1;
            
            if prop_num == prop {
                // Found it, return the next one
                addr += 1 + prop_len as usize;
                let next_size = self.memory.read_byte(addr);
                return next_size & 0x1F;
            }
            addr += 1 + prop_len as usize;
        }
    }
    
    fn put_object_property(&mut self, obj: u8, prop: u8, value: u16) {
        if obj == 0 { return; }
        let prop_addr = self.get_object_property_addr(obj, prop);
        if prop_addr == 0 {
            return;  // Property not found
        }
        
        let size_byte = self.memory.read_byte(prop_addr - 1);
        let prop_len = (size_byte / 32) + 1;
        
        if prop_len == 1 {
            self.memory.write_byte(prop_addr, value as u8);
        } else {
            self.memory.write_word(prop_addr, value);
        }
    }
    
    fn remove_object(&mut self, obj: u8) {
        if obj == 0 { return; }
        
        let parent = self.get_object_parent(obj);
        if parent == 0 { return; }
        
        let sibling = self.get_object_sibling(obj);
        let parent_child = self.get_object_child(parent);
        
        if parent_child == obj {
            self.set_object_child(parent, sibling);
        } else {
            let mut prev = parent_child;
            while prev != 0 {
                let next = self.get_object_sibling(prev);
                if next == obj {
                    self.set_object_sibling(prev, sibling);
                    break;
                }
                prev = next;
            }
        }
        
        self.set_object_parent(obj, 0);
        self.set_object_sibling(obj, 0);
    }
    
    fn insert_object(&mut self, obj: u8, dest: u8) {
        if obj == 0 { return; }
        
        // First remove from current parent
        self.remove_object(obj);
        
        if dest == 0 { return; }
        
        // Insert as first child of dest
        let old_child = self.get_object_child(dest);
        self.set_object_child(dest, obj);
        self.set_object_parent(obj, dest);
        self.set_object_sibling(obj, old_child);
    }
    
    fn show_status(&mut self) {
        // In v3, show the status line (location and score/moves or time)
        // For terminal mode, we'll just show a simple status
        let flags = self.memory.read_byte(0x01);
        let _is_time_game = flags & 0x02 != 0;
        
        // Get location name from global 0 (first global = the room)
        let globals_addr = self.memory.read_word(0x0C) as usize;
        let location_obj = self.memory.read_word(globals_addr) as u8;
        let location_name = self.get_object_short_name(location_obj);
        
        // Get score from global 1
        let score = self.memory.read_word(globals_addr + 2) as i16;
        
        // Get moves from global 2
        let moves = self.memory.read_word(globals_addr + 4);
        
        self.io.print(&format!("\n[{} | Score: {} | Moves: {}]\n", 
            location_name, score, moves));
    }
}

// Instruction forms
enum InstructionForm {
    Long,
    Short,
    Variable,
}

// Operand types
#[derive(Clone, Copy)]
enum OperandType {
    Large,
    Small,
    Variable,
}

// Operand values
#[derive(Clone)]
enum Operand {
    Large(u16),
    Small(u8),
    Variable(u16),
}

// Simple random number generator
static mut RAND_STATE: u32 = 12345;

fn rand_simple() -> u32 {
    unsafe {
        RAND_STATE = RAND_STATE.wrapping_mul(1103515245).wrapping_add(12345);
        (RAND_STATE / 65536) % 32768
    }
}
