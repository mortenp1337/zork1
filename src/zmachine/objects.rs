//! Object table handling for the Z-machine
//!
//! The Z-machine uses an object table for game entities (items, rooms, NPCs).

/// ObjectTable manages the Z-machine object tree
pub struct ObjectTable {
    // Object table base address
    base_addr: usize,
    // Version determines object structure size
    version: u8,
}

impl ObjectTable {
    /// Create a new object table
    pub fn new(base_addr: usize, version: u8) -> Self {
        ObjectTable { base_addr, version }
    }
    
    /// Get the address of an object entry
    pub fn object_addr(&self, obj: u8) -> usize {
        if obj == 0 {
            return 0;
        }
        
        // In v3:
        // - 31 default properties (2 bytes each) = 62 bytes
        // - Each object is 9 bytes
        let default_props_size = 31 * 2;
        let obj_entry_size = 9;
        
        self.base_addr + default_props_size + ((obj as usize - 1) * obj_entry_size)
    }
}
