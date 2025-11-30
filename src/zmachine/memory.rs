//! Z-Machine memory management

/// Z-Machine memory
pub struct Memory {
    /// Dynamic memory (can be modified)
    dynamic: Vec<u8>,
    /// Static memory (read-only during execution)
    static_mem: Vec<u8>,
    /// High memory (code area, read-only)
    high_mem: Vec<u8>,
    /// Original dynamic memory for restart
    original_dynamic: Vec<u8>,
    /// End of dynamic memory
    dynamic_end: usize,
    /// Start of high memory
    high_mem_start: usize,
}

impl Memory {
    /// Create a new Memory from story data
    pub fn new(story_data: &[u8]) -> Result<Self, String> {
        if story_data.len() < 64 {
            return Err("Story file too small".to_string());
        }
        
        // Read header values
        let version = story_data[0];
        if version < 1 || version > 3 {
            // We support v1-v3 for Zork I
            return Err(format!("Unsupported Z-machine version: {}", version));
        }
        
        // Dynamic memory ends at byte address in bytes 0x0E-0x0F
        let dynamic_end = ((story_data[0x0E] as usize) << 8) | (story_data[0x0F] as usize);
        
        // High memory starts at byte address in bytes 0x04-0x05
        let high_mem_start = ((story_data[0x04] as usize) << 8) | (story_data[0x05] as usize);
        
        let dynamic: Vec<u8> = story_data[..dynamic_end.min(story_data.len())].to_vec();
        let original_dynamic = dynamic.clone();
        
        let static_start = dynamic_end;
        let static_end = high_mem_start.min(story_data.len());
        let static_mem: Vec<u8> = if static_start < static_end {
            story_data[static_start..static_end].to_vec()
        } else {
            Vec::new()
        };
        
        let high_mem: Vec<u8> = if high_mem_start < story_data.len() {
            story_data[high_mem_start..].to_vec()
        } else {
            Vec::new()
        };
        
        Ok(Memory {
            dynamic,
            static_mem,
            high_mem,
            original_dynamic,
            dynamic_end,
            high_mem_start,
        })
    }
    
    /// Read a byte from memory
    pub fn read_byte(&self, addr: usize) -> u8 {
        if addr < self.dynamic.len() {
            self.dynamic[addr]
        } else if addr < self.high_mem_start {
            let offset = addr - self.dynamic_end;
            if offset < self.static_mem.len() {
                self.static_mem[offset]
            } else {
                0
            }
        } else {
            let offset = addr - self.high_mem_start;
            if offset < self.high_mem.len() {
                self.high_mem[offset]
            } else {
                0
            }
        }
    }
    
    /// Read a word (big-endian) from memory
    pub fn read_word(&self, addr: usize) -> u16 {
        let high = self.read_byte(addr) as u16;
        let low = self.read_byte(addr + 1) as u16;
        (high << 8) | low
    }
    
    /// Write a byte to memory (only dynamic memory is writable)
    pub fn write_byte(&mut self, addr: usize, value: u8) {
        if addr < self.dynamic.len() {
            self.dynamic[addr] = value;
        }
    }
    
    /// Write a word to memory (only dynamic memory is writable)
    pub fn write_word(&mut self, addr: usize, value: u16) {
        self.write_byte(addr, (value >> 8) as u8);
        self.write_byte(addr + 1, (value & 0xFF) as u8);
    }
    
    /// Restart the game (restore dynamic memory)
    pub fn restart(&mut self) {
        self.dynamic = self.original_dynamic.clone();
    }
    
    /// Get a slice of memory for direct access
    pub fn get_slice(&self, addr: usize, len: usize) -> Vec<u8> {
        let mut result = Vec::with_capacity(len);
        for i in 0..len {
            result.push(self.read_byte(addr + i));
        }
        result
    }
}
