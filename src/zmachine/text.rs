//! Z-machine text encoding/decoding
//!
//! The Z-machine uses a special encoding for text called "Z-strings" or "Z-characters".

use super::Memory;

/// TextEngine handles encoding and decoding of Z-machine text
pub struct TextEngine {
    version: u8,
}

// Standard Z-character to ASCII alphabet tables for version 3
const ALPHABET_A0: &[u8; 26] = b"abcdefghijklmnopqrstuvwxyz";
const ALPHABET_A1: &[u8; 26] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ";
const ALPHABET_A2: &[u8; 26] = b" \n0123456789.,!?_#'\"/\\-:()";

impl TextEngine {
    /// Create a new text engine for the given Z-machine version
    pub fn new(version: u8) -> Self {
        TextEngine { version }
    }
    
    /// Decode a Z-string at the given address
    /// Returns the decoded string and the number of bytes consumed
    pub fn decode(&self, memory: &Memory, addr: usize) -> (String, usize) {
        let mut result = String::new();
        let mut current_addr = addr;
        let mut alphabet = 0u8;  // 0, 1, or 2
        let mut pending_abbrev = None;
        let mut pending_zscii = None;
        
        loop {
            // Read a word (2 bytes, 3 Z-characters)
            let word = memory.read_word(current_addr);
            current_addr += 2;
            
            let end_bit = word & 0x8000 != 0;
            
            // Extract 3 Z-characters (5 bits each)
            let zchars = [
                ((word >> 10) & 0x1F) as u8,
                ((word >> 5) & 0x1F) as u8,
                (word & 0x1F) as u8,
            ];
            
            for zchar in zchars {
                if let Some(abbrev_type) = pending_abbrev {
                    // Handle abbreviation
                    let abbrev_index = (32 * (abbrev_type - 1) + zchar) as usize;
                    let abbrev_table = memory.read_word(0x18) as usize;
                    let abbrev_addr = memory.read_word(abbrev_table + abbrev_index * 2) as usize;
                    // Abbreviation addresses are word addresses
                    let (abbrev_text, _) = self.decode(memory, abbrev_addr * 2);
                    result.push_str(&abbrev_text);
                    pending_abbrev = None;
                    continue;
                }
                
                if let Some(high_bits) = pending_zscii {
                    // Second character of ZSCII pair
                    let zscii_char = (high_bits << 5) | zchar;
                    if zscii_char >= 32u8 && zscii_char < 127u8 {
                        result.push(zscii_char as char);
                    }
                    pending_zscii = None;
                    alphabet = 0;
                    continue;
                }
                
                match zchar {
                    0 => result.push(' '),
                    1..=3 => {
                        // Abbreviation (v2+)
                        pending_abbrev = Some(zchar);
                    }
                    4 => {
                        // Shift to A1
                        alphabet = 1;
                    }
                    5 => {
                        // Shift to A2
                        alphabet = 2;
                    }
                    6..=31 => {
                        let index = (zchar - 6) as usize;
                        let ch = match alphabet {
                            0 => ALPHABET_A0.get(index).copied().unwrap_or(b'?'),
                            1 => ALPHABET_A1.get(index).copied().unwrap_or(b'?'),
                            2 => {
                                if zchar == 6 {
                                    // 10-bit ZSCII escape sequence starts
                                    pending_zscii = Some(0);
                                    alphabet = 0;
                                    continue;
                                } else if zchar == 7 {
                                    // Newline
                                    alphabet = 0;
                                    b'\n'
                                } else {
                                    let a2_index = (zchar - 6) as usize;
                                    ALPHABET_A2.get(a2_index).copied().unwrap_or(b'?')
                                }
                            }
                            _ => b'?',
                        };
                        result.push(ch as char);
                        if alphabet != 0 {
                            alphabet = 0;  // Shift back to A0
                        }
                    }
                    _ => {}
                }
                
                // Check for 10-bit ZSCII continuation
                if pending_zscii == Some(0) {
                    pending_zscii = Some(zchar);
                }
            }
            
            if end_bit {
                break;
            }
        }
        
        (result, current_addr - addr)
    }
    
    /// Encode a word for dictionary lookup (max 6 Z-characters in v3)
    pub fn encode_word(&self, word: &str) -> Vec<u16> {
        let mut zchars = Vec::new();
        let max_zchars = 6;  // v3 uses 6 Z-characters (2 words)
        
        for ch in word.chars() {
            if zchars.len() >= max_zchars {
                break;
            }
            
            let ch = ch.to_ascii_lowercase();
            
            // Find character in alphabet A0 (a-z)
            if let Some(pos) = ALPHABET_A0.iter().position(|&c| c == ch as u8) {
                zchars.push((pos + 6) as u8);
            } else if let Some(pos) = ALPHABET_A2.iter().position(|&c| c == ch as u8) {
                // Found in A2 - need shift character first
                if zchars.len() + 1 < max_zchars {
                    zchars.push(5);  // Shift to A2
                    zchars.push((pos + 6) as u8);
                }
            }
            // Skip unknown characters
        }
        
        // Pad with 5s (shift character - standard padding)
        while zchars.len() < max_zchars {
            zchars.push(5);
        }
        
        // Pack into words
        let mut words = Vec::new();
        for i in (0..max_zchars).step_by(3) {
            let z0 = zchars.get(i).copied().unwrap_or(5) as u16;
            let z1 = zchars.get(i + 1).copied().unwrap_or(5) as u16;
            let z2 = zchars.get(i + 2).copied().unwrap_or(5) as u16;
            let word = (z0 << 10) | (z1 << 5) | z2;
            words.push(word);
        }
        
        // Set end bit on last word
        if let Some(last) = words.last_mut() {
            *last |= 0x8000;
        }
        
        words
    }
}
