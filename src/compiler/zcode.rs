//! Z-Code structure definitions
//!
//! Defines the Z-machine file format and related structures.

/// Z-machine header (64 bytes for v3)
pub struct ZHeader {
    /// Version number (1-8)
    pub version: u8,
    /// Flags 1
    pub flags1: u8,
    /// Release number
    pub release: u16,
    /// High memory base address
    pub high_mem_base: u16,
    /// Initial PC
    pub initial_pc: u16,
    /// Dictionary address
    pub dictionary: u16,
    /// Object table address
    pub object_table: u16,
    /// Global variables table address
    pub globals: u16,
    /// Static memory base address
    pub static_mem_base: u16,
    /// Flags 2
    pub flags2: u16,
    /// Serial number (6 ASCII chars)
    pub serial: [u8; 6],
    /// Abbreviations table address
    pub abbreviations: u16,
    /// File length (divided by 2 for v3)
    pub file_length: u16,
    /// Checksum
    pub checksum: u16,
}

impl ZHeader {
    /// Create a new header for v3
    pub fn new_v3() -> Self {
        ZHeader {
            version: 3,
            flags1: 0,
            release: 1,
            high_mem_base: 0,
            initial_pc: 0,
            dictionary: 0,
            object_table: 0,
            globals: 0,
            static_mem_base: 0,
            flags2: 0,
            serial: [b'0', b'0', b'0', b'0', b'0', b'1'],
            abbreviations: 0,
            file_length: 0,
            checksum: 0,
        }
    }
    
    /// Serialize header to bytes
    pub fn to_bytes(&self) -> [u8; 64] {
        let mut bytes = [0u8; 64];
        
        bytes[0] = self.version;
        bytes[1] = self.flags1;
        bytes[2] = (self.release >> 8) as u8;
        bytes[3] = (self.release & 0xFF) as u8;
        bytes[4] = (self.high_mem_base >> 8) as u8;
        bytes[5] = (self.high_mem_base & 0xFF) as u8;
        bytes[6] = (self.initial_pc >> 8) as u8;
        bytes[7] = (self.initial_pc & 0xFF) as u8;
        bytes[8] = (self.dictionary >> 8) as u8;
        bytes[9] = (self.dictionary & 0xFF) as u8;
        bytes[10] = (self.object_table >> 8) as u8;
        bytes[11] = (self.object_table & 0xFF) as u8;
        bytes[12] = (self.globals >> 8) as u8;
        bytes[13] = (self.globals & 0xFF) as u8;
        bytes[14] = (self.static_mem_base >> 8) as u8;
        bytes[15] = (self.static_mem_base & 0xFF) as u8;
        bytes[16] = (self.flags2 >> 8) as u8;
        bytes[17] = (self.flags2 & 0xFF) as u8;
        
        for (i, &b) in self.serial.iter().enumerate() {
            bytes[18 + i] = b;
        }
        
        bytes[24] = (self.abbreviations >> 8) as u8;
        bytes[25] = (self.abbreviations & 0xFF) as u8;
        bytes[26] = (self.file_length >> 8) as u8;
        bytes[27] = (self.file_length & 0xFF) as u8;
        bytes[28] = (self.checksum >> 8) as u8;
        bytes[29] = (self.checksum & 0xFF) as u8;
        
        bytes
    }
}

/// Z-machine object (9 bytes for v3)
#[derive(Clone, Default)]
pub struct ZObject {
    /// Attribute flags (4 bytes = 32 attributes)
    pub attributes: u32,
    /// Parent object number
    pub parent: u8,
    /// Sibling object number
    pub sibling: u8,
    /// Child object number
    pub child: u8,
    /// Property table address
    pub properties: u16,
}

impl ZObject {
    /// Serialize object to bytes (v3 format)
    pub fn to_bytes(&self) -> [u8; 9] {
        let mut bytes = [0u8; 9];
        
        bytes[0] = ((self.attributes >> 24) & 0xFF) as u8;
        bytes[1] = ((self.attributes >> 16) & 0xFF) as u8;
        bytes[2] = ((self.attributes >> 8) & 0xFF) as u8;
        bytes[3] = (self.attributes & 0xFF) as u8;
        bytes[4] = self.parent;
        bytes[5] = self.sibling;
        bytes[6] = self.child;
        bytes[7] = (self.properties >> 8) as u8;
        bytes[8] = (self.properties & 0xFF) as u8;
        
        bytes
    }
    
    /// Set an attribute
    pub fn set_attribute(&mut self, attr: u8) {
        if attr < 32 {
            self.attributes |= 1 << (31 - attr);
        }
    }
}

/// Dictionary entry
pub struct ZDictEntry {
    /// Encoded word (4 bytes for v3)
    pub encoded: [u8; 4],
    /// Data bytes
    pub data: Vec<u8>,
}

/// String encoder for Z-machine text
pub struct ZStringEncoder {
    /// Abbreviations
    abbreviations: Vec<String>,
}

impl ZStringEncoder {
    pub fn new() -> Self {
        ZStringEncoder {
            abbreviations: Vec::new(),
        }
    }
    
    /// Encode a string to Z-characters
    pub fn encode(&self, text: &str) -> Vec<u8> {
        let mut zchars: Vec<u8> = Vec::new();
        let alphabet_a0 = "abcdefghijklmnopqrstuvwxyz";
        let alphabet_a1 = "ABCDEFGHIJKLMNOPQRSTUVWXYZ";
        let alphabet_a2 = " \n0123456789.,!?_#'\"/\\-:()";
        
        for c in text.chars() {
            if c == ' ' {
                zchars.push(0);
            } else if let Some(pos) = alphabet_a0.find(c) {
                zchars.push((pos + 6) as u8);
            } else if let Some(pos) = alphabet_a1.find(c) {
                zchars.push(4); // Shift to A1
                zchars.push((pos + 6) as u8);
            } else if let Some(pos) = alphabet_a2.find(c) {
                zchars.push(5); // Shift to A2
                zchars.push((pos + 6) as u8);
            } else {
                // ZSCII escape for other characters
                let zscii = c as u8;
                zchars.push(5); // Shift to A2
                zchars.push(6); // ZSCII escape
                zchars.push((zscii >> 5) & 0x1F);
                zchars.push(zscii & 0x1F);
            }
        }
        
        // Pad to multiple of 3
        while zchars.len() % 3 != 0 {
            zchars.push(5); // Padding character
        }
        
        // Pack into words
        let mut result = Vec::new();
        let num_words = zchars.len() / 3;
        
        for i in 0..num_words {
            let c1 = zchars[i * 3] as u16;
            let c2 = zchars[i * 3 + 1] as u16;
            let c3 = zchars[i * 3 + 2] as u16;
            
            let mut word = (c1 << 10) | (c2 << 5) | c3;
            
            // Set end bit on last word
            if i == num_words - 1 {
                word |= 0x8000;
            }
            
            result.push((word >> 8) as u8);
            result.push((word & 0xFF) as u8);
        }
        
        // If empty, return a single end word
        if result.is_empty() {
            result.push(0x80);
            result.push(0xA5); // "   " with end bit
        }
        
        result
    }
    
    /// Encode a word for dictionary (exactly 4 bytes for v3)
    pub fn encode_word(&self, word: &str) -> [u8; 4] {
        let word = word.to_lowercase();
        let mut zchars = [5u8; 6]; // Pad with 5s
        
        let alphabet = "abcdefghijklmnopqrstuvwxyz";
        
        for (i, c) in word.chars().take(6).enumerate() {
            if let Some(pos) = alphabet.find(c) {
                zchars[i] = (pos + 6) as u8;
            }
        }
        
        // Pack into 2 words (4 bytes)
        let word1 = ((zchars[0] as u16) << 10) | ((zchars[1] as u16) << 5) | (zchars[2] as u16);
        let word2 = ((zchars[3] as u16) << 10) | ((zchars[4] as u16) << 5) | (zchars[5] as u16) | 0x8000;
        
        [
            (word1 >> 8) as u8,
            (word1 & 0xFF) as u8,
            (word2 >> 8) as u8,
            (word2 & 0xFF) as u8,
        ]
    }
}

impl Default for ZStringEncoder {
    fn default() -> Self {
        Self::new()
    }
}

/// Z-machine opcodes
#[allow(dead_code)]
pub mod opcodes {
    // 2OP opcodes
    pub const JE: u8 = 0x01;
    pub const JL: u8 = 0x02;
    pub const JG: u8 = 0x03;
    pub const DEC_CHK: u8 = 0x04;
    pub const INC_CHK: u8 = 0x05;
    pub const JIN: u8 = 0x06;
    pub const TEST: u8 = 0x07;
    pub const OR: u8 = 0x08;
    pub const AND: u8 = 0x09;
    pub const TEST_ATTR: u8 = 0x0A;
    pub const SET_ATTR: u8 = 0x0B;
    pub const CLEAR_ATTR: u8 = 0x0C;
    pub const STORE: u8 = 0x0D;
    pub const INSERT_OBJ: u8 = 0x0E;
    pub const LOADW: u8 = 0x0F;
    pub const LOADB: u8 = 0x10;
    pub const GET_PROP: u8 = 0x11;
    pub const GET_PROP_ADDR: u8 = 0x12;
    pub const GET_NEXT_PROP: u8 = 0x13;
    pub const ADD: u8 = 0x14;
    pub const SUB: u8 = 0x15;
    pub const MUL: u8 = 0x16;
    pub const DIV: u8 = 0x17;
    pub const MOD: u8 = 0x18;
    
    // 1OP opcodes (base 0x80)
    pub const JZ: u8 = 0x00;
    pub const GET_SIBLING: u8 = 0x01;
    pub const GET_CHILD: u8 = 0x02;
    pub const GET_PARENT: u8 = 0x03;
    pub const GET_PROP_LEN: u8 = 0x04;
    pub const INC: u8 = 0x05;
    pub const DEC: u8 = 0x06;
    pub const PRINT_ADDR: u8 = 0x07;
    pub const REMOVE_OBJ: u8 = 0x09;
    pub const PRINT_OBJ: u8 = 0x0A;
    pub const RET: u8 = 0x0B;
    pub const JUMP: u8 = 0x0C;
    pub const PRINT_PADDR: u8 = 0x0D;
    pub const LOAD: u8 = 0x0E;
    pub const NOT: u8 = 0x0F;
    
    // 0OP opcodes (base 0xB0)
    pub const RTRUE: u8 = 0x00;
    pub const RFALSE: u8 = 0x01;
    pub const PRINT: u8 = 0x02;
    pub const PRINT_RET: u8 = 0x03;
    pub const NOP: u8 = 0x04;
    pub const SAVE: u8 = 0x05;
    pub const RESTORE: u8 = 0x06;
    pub const RESTART: u8 = 0x07;
    pub const RET_POPPED: u8 = 0x08;
    pub const POP: u8 = 0x09;
    pub const QUIT: u8 = 0x0A;
    pub const NEW_LINE: u8 = 0x0B;
    pub const SHOW_STATUS: u8 = 0x0C;
    pub const VERIFY: u8 = 0x0D;
    
    // VAR opcodes (base 0xE0)
    pub const CALL: u8 = 0x00;
    pub const STOREW: u8 = 0x01;
    pub const STOREB: u8 = 0x02;
    pub const PUT_PROP: u8 = 0x03;
    pub const SREAD: u8 = 0x04;
    pub const PRINT_CHAR: u8 = 0x05;
    pub const PRINT_NUM: u8 = 0x06;
    pub const RANDOM: u8 = 0x07;
    pub const PUSH: u8 = 0x08;
    pub const PULL: u8 = 0x09;
    pub const SPLIT_WINDOW: u8 = 0x0A;
    pub const SET_WINDOW: u8 = 0x0B;
}
