//! Z-Machine Code Generator
//!
//! Generates Z-machine bytecode from the symbol table.

use std::collections::HashMap;
use crate::compiler::symbols::SymbolTable;
use crate::compiler::ast::*;
use crate::compiler::zcode::*;

/// Code generator for Z-machine
pub struct CodeGenerator<'a> {
    symbols: &'a SymbolTable,
    version: u8,
    
    // Output buffers
    code: Vec<u8>,
    strings: Vec<u8>,
    
    // Address tracking
    object_table_addr: u16,
    property_table_addr: u16,
    globals_addr: u16,
    dictionary_addr: u16,
    high_mem_addr: u16,
    
    // Mappings
    object_numbers: HashMap<String, u16>,
    routine_addrs: HashMap<String, u16>,
    global_numbers: HashMap<String, u8>,
    string_addrs: HashMap<String, u16>,
    
    // String encoder
    encoder: ZStringEncoder,
}

impl<'a> CodeGenerator<'a> {
    /// Create a new code generator
    pub fn new(symbols: &'a SymbolTable, version: u8) -> Self {
        CodeGenerator {
            symbols,
            version,
            code: Vec::new(),
            strings: Vec::new(),
            object_table_addr: 0,
            property_table_addr: 0,
            globals_addr: 0,
            dictionary_addr: 0,
            high_mem_addr: 0,
            object_numbers: HashMap::new(),
            routine_addrs: HashMap::new(),
            global_numbers: HashMap::new(),
            string_addrs: HashMap::new(),
            encoder: ZStringEncoder::new(),
        }
    }
    
    /// Generate Z-machine code
    pub fn generate(&mut self) -> Result<Vec<u8>, String> {
        // Calculate memory layout
        self.calculate_layout();
        
        // Generate header
        let mut output = vec![0u8; 64];
        
        // Generate abbreviations table (simplified - no abbreviations for now)
        let abbrev_addr = 64;
        let abbrev_table = self.generate_abbreviations();
        
        // Generate object table
        self.object_table_addr = (abbrev_addr + abbrev_table.len()) as u16;
        // Align to word boundary
        while (self.object_table_addr as usize) % 2 != 0 {
            self.object_table_addr += 1;
        }
        let object_table = self.generate_object_table()?;
        
        // Generate property tables
        self.property_table_addr = self.object_table_addr + object_table.len() as u16;
        let property_tables = self.generate_property_tables()?;
        
        // Generate globals table
        self.globals_addr = self.property_table_addr + property_tables.len() as u16;
        // Align to word boundary
        while (self.globals_addr as usize) % 2 != 0 {
            self.globals_addr += 1;
        }
        let globals_table = self.generate_globals_table();
        
        // Generate dictionary
        self.dictionary_addr = self.globals_addr + globals_table.len() as u16;
        let dictionary = self.generate_dictionary();
        
        // Calculate high memory start (where code begins)
        self.high_mem_addr = self.dictionary_addr + dictionary.len() as u16;
        // Align to packed address boundary
        while (self.high_mem_addr as usize) % 2 != 0 {
            self.high_mem_addr += 1;
        }
        
        // Generate routines
        let routines = self.generate_routines()?;
        
        // Generate static strings
        let static_strings = self.generate_static_strings();
        
        // Build final output
        let static_mem_base = self.high_mem_addr;
        let file_length = 64 + abbrev_table.len() + 
            (self.object_table_addr as usize - abbrev_addr - abbrev_table.len()) +
            object_table.len() + property_tables.len() +
            (self.globals_addr as usize - self.property_table_addr as usize - property_tables.len()) +
            globals_table.len() + dictionary.len() +
            (self.high_mem_addr as usize - self.dictionary_addr as usize - dictionary.len()) +
            routines.len() + static_strings.len();
        
        // Create header
        let mut header = ZHeader::new_v3();
        header.high_mem_base = self.high_mem_addr;
        header.initial_pc = self.get_main_routine_addr();
        header.dictionary = self.dictionary_addr;
        header.object_table = self.object_table_addr;
        header.globals = self.globals_addr;
        header.static_mem_base = static_mem_base;
        header.abbreviations = abbrev_addr as u16;
        header.file_length = (file_length / 2) as u16;
        
        let header_bytes = header.to_bytes();
        output[..64].copy_from_slice(&header_bytes);
        
        // Append all sections
        output.extend(&abbrev_table);
        
        // Pad to object table
        while output.len() < self.object_table_addr as usize {
            output.push(0);
        }
        output.extend(&object_table);
        output.extend(&property_tables);
        
        // Pad to globals
        while output.len() < self.globals_addr as usize {
            output.push(0);
        }
        output.extend(&globals_table);
        output.extend(&dictionary);
        
        // Pad to high memory
        while output.len() < self.high_mem_addr as usize {
            output.push(0);
        }
        output.extend(&routines);
        output.extend(&static_strings);
        
        // Calculate and update checksum
        let checksum = self.calculate_checksum(&output);
        output[28] = (checksum >> 8) as u8;
        output[29] = (checksum & 0xFF) as u8;
        
        Ok(output)
    }
    
    /// Calculate memory layout
    fn calculate_layout(&mut self) {
        // Assign object numbers
        let mut obj_num: u16 = 1;
        
        // First assign rooms (they're also objects)
        for name in self.symbols.rooms.keys() {
            self.object_numbers.insert(name.clone(), obj_num);
            obj_num += 1;
        }
        
        // Then assign objects
        for name in self.symbols.objects.keys() {
            if !self.object_numbers.contains_key(name) {
                self.object_numbers.insert(name.clone(), obj_num);
                obj_num += 1;
            }
        }
        
        // Assign global variable numbers
        let mut global_num: u8 = 16; // Globals start at 16 in v3
        for name in self.symbols.globals.keys() {
            self.global_numbers.insert(name.clone(), global_num);
            if global_num < 255 {
                global_num += 1;
            }
        }
    }
    
    /// Generate abbreviations table
    fn generate_abbreviations(&self) -> Vec<u8> {
        // For v3, we need 96 abbreviation entries (32 * 3)
        // Each entry is a word address (2 bytes)
        // For now, generate empty abbreviations
        let mut table = Vec::new();
        
        // 96 entries, each pointing to an empty string
        for _ in 0..96 {
            table.push(0);
            table.push(0);
        }
        
        table
    }
    
    /// Generate object table
    fn generate_object_table(&mut self) -> Result<Vec<u8>, String> {
        let mut table = Vec::new();
        
        // Property defaults (31 words for v3)
        for _ in 0..31 {
            table.push(0);
            table.push(0);
        }
        
        // Object entries (9 bytes each for v3)
        let num_objects = self.object_numbers.len();
        
        // Pre-allocate space for all objects
        let objects_start = table.len();
        for _ in 0..num_objects {
            table.extend(&[0u8; 9]);
        }
        
        // Fill in object data
        for (name, &obj_num) in &self.object_numbers {
            let mut obj = ZObject::default();
            
            // Get object definition
            if let Some(obj_def) = self.symbols.objects.get(name) {
                // Set attributes from FLAGS property
                if let Some(Property::Flags(flags)) = obj_def.properties.get("FLAGS") {
                    for flag in flags {
                        if let Some(&attr) = self.symbols.flags.get(flag) {
                            obj.set_attribute(attr);
                        }
                    }
                }
                
                // Set parent from IN property
                if let Some(Property::In(parent)) = obj_def.properties.get("IN") {
                    if let Some(&parent_num) = self.object_numbers.get(parent) {
                        obj.parent = parent_num as u8;
                    }
                }
            } else if let Some(room_def) = self.symbols.rooms.get(name) {
                // Set attributes from FLAGS property
                if let Some(Property::Flags(flags)) = room_def.properties.get("FLAGS") {
                    for flag in flags {
                        if let Some(&attr) = self.symbols.flags.get(flag) {
                            obj.set_attribute(attr);
                        }
                    }
                }
            }
            
            // Property table address will be filled in later
            obj.properties = 0;
            
            // Write object to table
            let obj_offset = objects_start + ((obj_num as usize - 1) * 9);
            let obj_bytes = obj.to_bytes();
            table[obj_offset..obj_offset + 9].copy_from_slice(&obj_bytes);
        }
        
        Ok(table)
    }
    
    /// Generate property tables for all objects
    fn generate_property_tables(&mut self) -> Result<Vec<u8>, String> {
        let mut tables = Vec::new();
        
        for (name, &_obj_num) in &self.object_numbers.clone() {
            let prop_addr = self.property_table_addr + tables.len() as u16;
            
            // Short name (encoded Z-string length + string)
            let short_name = self.get_object_short_name(name);
            let encoded_name = self.encoder.encode(&short_name);
            let name_words = encoded_name.len() / 2;
            
            tables.push(name_words as u8);
            tables.extend(&encoded_name);
            
            // Properties (for now, just end marker)
            tables.push(0); // End of properties
            
            // Update object's property pointer
            // (This would need to update the object table)
        }
        
        Ok(tables)
    }
    
    /// Get object's short name
    fn get_object_short_name(&self, name: &str) -> String {
        if let Some(obj_def) = self.symbols.objects.get(name) {
            if let Some(Property::String(desc)) = obj_def.properties.get("DESC") {
                return desc.clone();
            }
        }
        if let Some(room_def) = self.symbols.rooms.get(name) {
            if let Some(Property::String(desc)) = room_def.properties.get("DESC") {
                return desc.clone();
            }
        }
        name.to_lowercase().replace('_', " ").replace('-', " ")
    }
    
    /// Generate globals table
    fn generate_globals_table(&self) -> Vec<u8> {
        // 240 global variables, 2 bytes each
        let mut table = vec![0u8; 480];
        
        for (name, value) in &self.symbols.globals {
            if let Some(&global_num) = self.global_numbers.get(name) {
                let offset = ((global_num - 16) as usize) * 2;
                if offset < 480 {
                    let val = self.evaluate_constant_expr(value);
                    table[offset] = (val >> 8) as u8;
                    table[offset + 1] = (val & 0xFF) as u8;
                }
            }
        }
        
        table
    }
    
    /// Evaluate a constant expression
    fn evaluate_constant_expr(&self, expr: &Expr) -> u16 {
        match expr {
            Expr::Number(n) => *n as u16,
            Expr::True => 1,
            Expr::False => 0,
            Expr::Atom(name) => {
                // Look up in constants
                if let Some(val) = self.symbols.constants.get(name) {
                    self.evaluate_constant_expr(val)
                } else if let Some(&obj_num) = self.object_numbers.get(name) {
                    obj_num
                } else {
                    0
                }
            }
            Expr::GVal(name) => {
                if let Some(&global_num) = self.global_numbers.get(name) {
                    global_num as u16
                } else {
                    0
                }
            }
            _ => 0,
        }
    }
    
    /// Generate dictionary
    fn generate_dictionary(&self) -> Vec<u8> {
        let mut dict = Vec::new();
        
        // Word separators
        let separators = [b'.', b',', b'"', b'\''];
        dict.push(separators.len() as u8);
        dict.extend(&separators);
        
        // Entry length (4 bytes word + data bytes)
        let entry_len = 7u8; // 4 bytes encoded word + 3 data bytes
        dict.push(entry_len);
        
        // Collect all words
        let mut words: Vec<String> = Vec::new();
        
        // Add object synonyms
        for obj in self.symbols.objects.values() {
            if let Some(Property::Synonym(syns)) = obj.properties.get("SYNONYM") {
                words.extend(syns.iter().cloned());
            }
        }
        
        // Add room synonyms
        for room in self.symbols.rooms.values() {
            if let Some(Property::Synonym(syns)) = room.properties.get("SYNONYM") {
                words.extend(syns.iter().cloned());
            }
        }
        
        // Sort and deduplicate
        words.sort();
        words.dedup();
        
        // Number of entries
        let num_entries = words.len() as u16;
        dict.push((num_entries >> 8) as u8);
        dict.push((num_entries & 0xFF) as u8);
        
        // Dictionary entries
        for word in words {
            let encoded = self.encoder.encode_word(&word);
            dict.extend(&encoded);
            // Data bytes (flags, verb number, etc.)
            dict.extend(&[0u8; 3]);
        }
        
        dict
    }
    
    /// Generate all routines
    fn generate_routines(&mut self) -> Result<Vec<u8>, String> {
        let mut code = Vec::new();
        
        // Generate a simple main routine that prints welcome and quits
        // This is a minimal implementation
        
        // Main routine (GO routine)
        let main_addr = self.high_mem_addr + code.len() as u16;
        self.routine_addrs.insert("GO".to_string(), main_addr / 2);
        
        // Routine header: 0 local variables
        code.push(0);
        
        // Print a welcome message
        // print "Welcome to Zork I!"
        code.push(0xB2); // print
        let msg = "ZORK I: The Great Underground Empire\nCopyright (c) 1980 Infocom, Inc. All rights reserved.\n\nCompiled with Rust ZIL Compiler\n";
        let encoded_msg = self.encoder.encode(msg);
        code.extend(&encoded_msg);
        
        // new_line
        code.push(0xBB);
        
        // quit
        code.push(0xBA);
        
        Ok(code)
    }
    
    /// Generate static strings
    fn generate_static_strings(&self) -> Vec<u8> {
        Vec::new()
    }
    
    /// Get main routine address
    fn get_main_routine_addr(&self) -> u16 {
        if let Some(&addr) = self.routine_addrs.get("GO") {
            // Packed address
            addr * 2
        } else {
            self.high_mem_addr
        }
    }
    
    /// Calculate checksum
    fn calculate_checksum(&self, data: &[u8]) -> u16 {
        let mut sum: u32 = 0;
        for (i, &byte) in data.iter().enumerate() {
            if i >= 64 {
                sum = sum.wrapping_add(byte as u32);
            }
        }
        (sum & 0xFFFF) as u16
    }
}
