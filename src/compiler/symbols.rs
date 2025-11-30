//! Symbol Table for ZIL Compiler
//!
//! Tracks all defined symbols: constants, globals, objects, rooms, routines, etc.

use std::collections::HashMap;
use crate::compiler::ast::*;

/// Symbol table for the ZIL compiler
pub struct SymbolTable {
    /// Constants (name -> value)
    pub constants: HashMap<String, Expr>,
    /// Global variables (name -> initial value)
    pub globals: HashMap<String, Expr>,
    /// Objects
    pub objects: HashMap<String, ObjectDef>,
    /// Rooms
    pub rooms: HashMap<String, RoomDef>,
    /// Routines
    pub routines: HashMap<String, RoutineDef>,
    /// Macros
    pub macros: HashMap<String, MacroDef>,
    /// Syntax definitions
    pub syntax: Vec<SyntaxDef>,
    /// Direction property names
    pub directions: Vec<String>,
    /// Object flags (attributes)
    pub flags: HashMap<String, u8>,
    /// Property definitions
    pub properties: HashMap<String, PropertyDef>,
    /// Verbs
    pub verbs: HashMap<String, u8>,
    /// Prepositions
    pub prepositions: HashMap<String, u8>,
    /// Action routines
    pub actions: Vec<String>,
    /// Buzz words (ignored in parsing)
    pub buzz_words: Vec<String>,
}

/// Macro definition
#[derive(Debug, Clone)]
pub struct MacroDef {
    pub name: String,
    pub args: Vec<String>,
    pub body: Vec<Expr>,
}

/// Property definition
#[derive(Debug, Clone)]
pub struct PropertyDef {
    pub name: String,
    pub number: u8,
    pub default: i32,
}

impl SymbolTable {
    /// Create a new empty symbol table
    pub fn new() -> Self {
        let mut table = SymbolTable {
            constants: HashMap::new(),
            globals: HashMap::new(),
            objects: HashMap::new(),
            rooms: HashMap::new(),
            routines: HashMap::new(),
            macros: HashMap::new(),
            syntax: Vec::new(),
            directions: Vec::new(),
            flags: HashMap::new(),
            properties: HashMap::new(),
            verbs: HashMap::new(),
            prepositions: HashMap::new(),
            actions: Vec::new(),
            buzz_words: Vec::new(),
        };
        
        // Define standard flags (attributes)
        table.define_standard_flags();
        
        // Define standard properties
        table.define_standard_properties();
        
        table
    }
    
    /// Define standard Z-machine flags/attributes
    fn define_standard_flags(&mut self) {
        let flags = [
            "LIGHTBIT", "ONBIT", "OPENBIT", "TOUCHBIT", "TAKEBIT",
            "CONTBIT", "DOORBIT", "FOODBIT", "DRINKBIT", "WEARBIT",
            "DEVICEBIT", "SURFACEBIT", "FLAMEBIT", "TOOLBIT", "WEAPONBIT",
            "VOWELBIT", "TRANSBIT", "VEHBIT", "SACREDBIT", "BURNBIT",
            "READBIT", "CLIMBBIT", "FEMALEBIT", "PLURALBIT", "TRYTAKEBIT",
            "SEARCHBIT", "INVISIBLE", "NDESCBIT", "RMUNGBIT", "RLANDBIT",
            "ACTORBIT", "FIGHTBIT", "STAGGERED", "MAGICBIT", "NARRATEBIT",
        ];
        
        for (i, flag) in flags.iter().enumerate() {
            if i < 32 {
                self.flags.insert(flag.to_string(), i as u8);
            }
        }
    }
    
    /// Define standard Z-machine properties
    fn define_standard_properties(&mut self) {
        // Standard property numbers for v3
        let props = [
            ("DESC", 1, 0),
            ("SYNONYM", 2, 0),
            ("ADJECTIVE", 3, 0),
            ("ACTION", 4, 0),
            ("DESCFCN", 5, 0),
            ("GLOBAL", 6, 0),
            ("FDESC", 7, 0),
            ("LDESC", 8, 0),
            ("PSEUDO", 9, 0),
            ("CONTFCN", 10, 0),
            ("CAPACITY", 11, 0),
            ("SIZE", 12, 5),
            ("VALUE", 13, 0),
            ("TVALUE", 14, 0),
            ("TEXT", 15, 0),
            ("VTYPE", 16, 0),
            ("STRENGTH", 17, 0),
        ];
        
        for (name, num, default) in props {
            self.properties.insert(name.to_string(), PropertyDef {
                name: name.to_string(),
                number: num,
                default,
            });
        }
    }
    
    /// Define a constant
    pub fn define_constant(&mut self, name: &str, value: Expr) {
        self.constants.insert(name.to_string(), value);
    }
    
    /// Define a global variable
    pub fn define_global(&mut self, name: &str, value: Expr) {
        self.globals.insert(name.to_string(), value);
    }
    
    /// Set a global variable value
    pub fn set_global(&mut self, name: &str, value: Expr) {
        self.globals.insert(name.to_string(), value);
    }
    
    /// Define an object
    pub fn define_object(&mut self, obj: ObjectDef) {
        self.objects.insert(obj.name.clone(), obj);
    }
    
    /// Define a room
    pub fn define_room(&mut self, room: RoomDef) {
        self.rooms.insert(room.name.clone(), room);
    }
    
    /// Define a routine
    pub fn define_routine(&mut self, routine: RoutineDef) {
        self.routines.insert(routine.name.clone(), routine);
    }
    
    /// Define a macro
    pub fn define_macro(&mut self, name: &str, args: Vec<String>, body: Vec<Expr>) {
        self.macros.insert(name.to_string(), MacroDef {
            name: name.to_string(),
            args,
            body,
        });
    }
    
    /// Define a syntax entry
    pub fn define_syntax(&mut self, syntax: SyntaxDef) {
        self.syntax.push(syntax);
    }
    
    /// Define directions
    pub fn define_directions(&mut self, dirs: Vec<String>) {
        self.directions = dirs;
        
        // Also add each direction as a property
        for (i, dir) in self.directions.iter().enumerate() {
            let prop_num = 17 + i as u8; // Start after standard properties
            self.properties.insert(dir.clone(), PropertyDef {
                name: dir.clone(),
                number: prop_num,
                default: 0,
            });
        }
    }
    
    /// Define a property
    pub fn define_property(&mut self, name: &str, default: i32) {
        let next_num = self.properties.len() as u8 + 1;
        if !self.properties.contains_key(name) {
            self.properties.insert(name.to_string(), PropertyDef {
                name: name.to_string(),
                number: next_num,
                default,
            });
        }
    }
    
    /// Get property number
    pub fn get_property_number(&self, name: &str) -> Option<u8> {
        self.properties.get(name).map(|p| p.number)
    }
    
    /// Get flag number
    pub fn get_flag_number(&self, name: &str) -> Option<u8> {
        self.flags.get(name).copied()
    }
    
    /// Look up a symbol
    pub fn lookup(&self, name: &str) -> Option<SymbolKind> {
        if self.constants.contains_key(name) {
            return Some(SymbolKind::Constant);
        }
        if self.globals.contains_key(name) {
            return Some(SymbolKind::Global);
        }
        if self.objects.contains_key(name) {
            return Some(SymbolKind::Object);
        }
        if self.rooms.contains_key(name) {
            return Some(SymbolKind::Room);
        }
        if self.routines.contains_key(name) {
            return Some(SymbolKind::Routine);
        }
        None
    }
    
    /// Get all objects (including rooms)
    pub fn all_objects(&self) -> Vec<&str> {
        let mut result: Vec<&str> = self.objects.keys().map(|s| s.as_str()).collect();
        result.extend(self.rooms.keys().map(|s| s.as_str()));
        result
    }
    
    /// Get object index (1-based)
    pub fn get_object_index(&self, name: &str) -> Option<u16> {
        let all = self.all_objects();
        all.iter().position(|&n| n == name).map(|i| (i + 1) as u16)
    }
}

impl Default for SymbolTable {
    fn default() -> Self {
        Self::new()
    }
}

/// Kind of symbol
#[derive(Debug, Clone, PartialEq)]
pub enum SymbolKind {
    Constant,
    Global,
    Object,
    Room,
    Routine,
    Macro,
    Flag,
    Property,
}
