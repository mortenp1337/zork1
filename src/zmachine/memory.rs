//! Memory management for the Z-machine
//!
//! The Z-machine has a flat memory model with the story file loaded at address 0.

use super::ZMachineError;

/// Memory represents the Z-machine's address space
pub struct Memory {
    data: Vec<u8>,
    dynamic_end: usize,  // End of dynamic memory (read-write)
}

impl Memory {
    /// Create a new Memory from story file data
    pub fn new(data: Vec<u8>) -> Result<Self, ZMachineError> {
        if data.len() < 64 {
            return Err(ZMachineError::new("Story file too small for header"));
        }
        
        // Static memory base is at offset 0x0E (word)
        let static_base = ((data[0x0E] as usize) << 8) | (data[0x0F] as usize);
        
        Ok(Memory {
            data,
            dynamic_end: static_base,
        })
    }
    
    /// Read a byte from memory
    pub fn read_byte(&self, addr: usize) -> u8 {
        if addr < self.data.len() {
            self.data[addr]
        } else {
            0
        }
    }
    
    /// Read a word (big-endian) from memory
    pub fn read_word(&self, addr: usize) -> u16 {
        if addr + 1 < self.data.len() {
            ((self.data[addr] as u16) << 8) | (self.data[addr + 1] as u16)
        } else {
            0
        }
    }
    
    /// Write a byte to memory
    pub fn write_byte(&mut self, addr: usize, value: u8) {
        if addr < self.data.len() {
            self.data[addr] = value;
        }
    }
    
    /// Write a word (big-endian) to memory
    pub fn write_word(&mut self, addr: usize, value: u16) {
        if addr + 1 < self.data.len() {
            self.data[addr] = (value >> 8) as u8;
            self.data[addr + 1] = (value & 0xFF) as u8;
        }
    }
    
    /// Get a slice of memory
    pub fn slice(&self, start: usize, len: usize) -> &[u8] {
        let end = (start + len).min(self.data.len());
        &self.data[start..end]
    }
    
    /// Get the length of memory
    pub fn len(&self) -> usize {
        self.data.len()
    }
    
    /// Check if memory is empty
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }
}
