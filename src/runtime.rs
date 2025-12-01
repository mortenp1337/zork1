/// ZIL Runtime - Executes ZIL code
use crate::ast::*;
use std::collections::HashMap;
use std::io::{self, Write, BufRead};

// Object hierarchy management
pub struct ObjectTree {
    pub objects: HashMap<String, usize>,
    pub parents: HashMap<usize, Option<usize>>,
    pub first_children: HashMap<usize, Option<usize>>,
    pub siblings: HashMap<usize, Option<usize>>,
}

impl ObjectTree {
    pub fn new() -> Self {
        ObjectTree {
            objects: HashMap::new(),
            parents: HashMap::new(),
            first_children: HashMap::new(),
            siblings: HashMap::new(),
        }
    }

    pub fn insert(&mut self, name: &str, id: usize) {
        self.objects.insert(name.to_uppercase(), id);
        self.parents.insert(id, None);
        self.first_children.insert(id, None);
        self.siblings.insert(id, None);
    }

    pub fn get_id(&self, name: &str) -> Option<usize> {
        self.objects.get(&name.to_uppercase()).copied()
    }

    pub fn set_parent(&mut self, child_id: usize, parent_id: Option<usize>) {
        self.parents.insert(child_id, parent_id);
    }

    pub fn get_parent(&self, id: usize) -> Option<usize> {
        self.parents.get(&id).copied().flatten()
    }

    pub fn move_object(&mut self, obj_id: usize, new_parent_id: usize) {
        // Remove from old parent
        if let Some(old_parent) = self.get_parent(obj_id) {
            self.remove_child(old_parent, obj_id);
        }
        
        // Add to new parent
        self.set_parent(obj_id, Some(new_parent_id));
        self.add_child(new_parent_id, obj_id);
    }

    fn remove_child(&mut self, parent_id: usize, child_id: usize) {
        let first = self.first_children.get(&parent_id).copied().flatten();
        if first == Some(child_id) {
            let sibling = self.siblings.get(&child_id).copied().flatten();
            self.first_children.insert(parent_id, sibling);
        } else {
            // Find and remove from sibling chain
            let mut current = first;
            while let Some(curr_id) = current {
                let sibling = self.siblings.get(&curr_id).copied().flatten();
                if sibling == Some(child_id) {
                    let next = self.siblings.get(&child_id).copied().flatten();
                    self.siblings.insert(curr_id, next);
                    break;
                }
                current = sibling;
            }
        }
        self.siblings.insert(child_id, None);
    }

    fn add_child(&mut self, parent_id: usize, child_id: usize) {
        let first = self.first_children.get(&parent_id).copied().flatten();
        self.siblings.insert(child_id, first);
        self.first_children.insert(parent_id, Some(child_id));
    }

    pub fn get_first_child(&self, id: usize) -> Option<usize> {
        self.first_children.get(&id).copied().flatten()
    }

    pub fn get_sibling(&self, id: usize) -> Option<usize> {
        self.siblings.get(&id).copied().flatten()
    }

    pub fn in_(&self, obj_id: usize, container_id: usize) -> bool {
        self.get_parent(obj_id) == Some(container_id)
    }
}

pub struct Runtime {
    pub globals: HashMap<String, ZilValue>,
    pub objects: Vec<ZilObject>,
    pub object_tree: ObjectTree,
    pub object_flags: HashMap<usize, Vec<String>>,
    pub routines: HashMap<String, ZilRoutine>,
    pub synonyms: HashMap<String, String>,
    pub directions: Vec<String>,
    pub buzz_words: Vec<String>,
    pub syntaxes: Vec<ZilSyntax>,
    pub property_defaults: HashMap<String, ZilValue>,
    
    // Game state
    pub score: i64,
    pub moves: i64,
    pub lit: bool,
    pub dead: bool,
    pub quit_flag: bool,
    
    // Current command parsing
    pub prsa: String,      // Current action verb
    pub prso: Option<usize>, // Direct object
    pub prsi: Option<usize>, // Indirect object
    
    // Call stack for local variables
    pub call_stack: Vec<HashMap<String, ZilValue>>,
    
    // Random state
    rng_state: u64,
}

impl Runtime {
    pub fn new() -> Self {
        Runtime {
            globals: HashMap::new(),
            objects: Vec::new(),
            object_tree: ObjectTree::new(),
            object_flags: HashMap::new(),
            routines: HashMap::new(),
            synonyms: HashMap::new(),
            directions: Vec::new(),
            buzz_words: Vec::new(),
            syntaxes: Vec::new(),
            property_defaults: HashMap::new(),
            score: 0,
            moves: 0,
            lit: true,
            dead: false,
            quit_flag: false,
            prsa: String::new(),
            prso: None,
            prsi: None,
            call_stack: Vec::new(),
            rng_state: 12345,
        }
    }

    pub fn load_definitions(&mut self, definitions: &[ZilDefinition]) {
        // First pass: register all objects and rooms
        for def in definitions {
            match def {
                ZilDefinition::Object(obj) | ZilDefinition::Room(obj) => {
                    let id = self.objects.len();
                    self.object_tree.insert(&obj.name, id);
                    self.object_flags.insert(id, obj.flags.clone());
                    self.objects.push(obj.clone());
                }
                ZilDefinition::Routine(routine) | ZilDefinition::Macro(routine) => {
                    self.routines.insert(routine.name.clone(), routine.clone());
                }
                ZilDefinition::Global(global) => {
                    self.globals.insert(global.name.clone(), global.value.clone());
                }
                ZilDefinition::Constant(constant) => {
                    self.globals.insert(constant.name.clone(), constant.value.clone());
                }
                ZilDefinition::Synonym(primary, synonyms) => {
                    for syn in synonyms {
                        self.synonyms.insert(syn.clone(), primary.clone());
                    }
                }
                ZilDefinition::Directions(dirs) => {
                    self.directions = dirs.clone();
                }
                ZilDefinition::Buzz(words) => {
                    self.buzz_words.extend(words.clone());
                }
                ZilDefinition::Syntax(syntax) => {
                    self.syntaxes.push(syntax.clone());
                }
                ZilDefinition::PropertyDefault(name, value) => {
                    self.property_defaults.insert(name.clone(), value.clone());
                }
                _ => {}
            }
        }

        // Second pass: set up object hierarchy
        for id in 0..self.objects.len() {
            if let Some(parent_name) = &self.objects[id].parent.clone() {
                if let Some(parent_id) = self.object_tree.get_id(parent_name) {
                    self.object_tree.move_object(id, parent_id);
                }
            }
        }

        // Initialize default globals
        if !self.globals.contains_key("HERE") {
            self.globals.insert("HERE".to_string(), ZilValue::Number(0));
        }
        if !self.globals.contains_key("WINNER") {
            self.globals.insert("WINNER".to_string(), ZilValue::Number(0));
        }
        if !self.globals.contains_key("LIT") {
            self.globals.insert("LIT".to_string(), ZilValue::Number(1));
        }
        if !self.globals.contains_key("SCORE") {
            self.globals.insert("SCORE".to_string(), ZilValue::Number(0));
        }
        if !self.globals.contains_key("MOVES") {
            self.globals.insert("MOVES".to_string(), ZilValue::Number(0));
        }
    }

    // Random number generator
    fn random(&mut self, max: i64) -> i64 {
        self.rng_state = self.rng_state.wrapping_mul(6364136223846793005).wrapping_add(1);
        ((self.rng_state >> 33) as i64 % max) + 1
    }

    pub fn get_object_id(&self, name: &str) -> Option<usize> {
        self.object_tree.get_id(name)
    }

    pub fn get_object(&self, id: usize) -> Option<&ZilObject> {
        self.objects.get(id)
    }

    pub fn get_object_mut(&mut self, id: usize) -> Option<&mut ZilObject> {
        self.objects.get_mut(id)
    }

    pub fn get_property(&self, obj_id: usize, prop: &str) -> ZilValue {
        if let Some(obj) = self.objects.get(obj_id) {
            if let Some(val) = obj.properties.get(prop) {
                return val.clone();
            }
        }
        if let Some(default) = self.property_defaults.get(prop) {
            return default.clone();
        }
        ZilValue::Nil
    }

    pub fn set_property(&mut self, obj_id: usize, prop: &str, value: ZilValue) {
        if let Some(obj) = self.objects.get_mut(obj_id) {
            obj.properties.insert(prop.to_string(), value);
        }
    }

    pub fn has_flag(&self, obj_id: usize, flag: &str) -> bool {
        self.object_flags.get(&obj_id).map_or(false, |flags| flags.contains(&flag.to_uppercase()))
    }

    pub fn set_flag(&mut self, obj_id: usize, flag: &str) {
        if let Some(flags) = self.object_flags.get_mut(&obj_id) {
            let flag_upper = flag.to_uppercase();
            if !flags.contains(&flag_upper) {
                flags.push(flag_upper);
            }
        }
    }

    pub fn clear_flag(&mut self, obj_id: usize, flag: &str) {
        if let Some(flags) = self.object_flags.get_mut(&obj_id) {
            flags.retain(|f| f != &flag.to_uppercase());
        }
    }

    pub fn move_object(&mut self, obj_id: usize, dest_id: usize) {
        self.object_tree.move_object(obj_id, dest_id);
    }

    pub fn get_local(&self, name: &str) -> ZilValue {
        if let Some(frame) = self.call_stack.last() {
            if let Some(val) = frame.get(&name.to_uppercase()) {
                return val.clone();
            }
        }
        ZilValue::Nil
    }

    pub fn set_local(&mut self, name: &str, value: ZilValue) {
        if let Some(frame) = self.call_stack.last_mut() {
            frame.insert(name.to_uppercase(), value);
        }
    }

    pub fn get_global(&self, name: &str) -> ZilValue {
        self.globals.get(&name.to_uppercase()).cloned().unwrap_or(ZilValue::Nil)
    }

    pub fn set_global(&mut self, name: &str, value: ZilValue) {
        self.globals.insert(name.to_uppercase(), value);
    }

    pub fn eval(&mut self, value: &ZilValue) -> ZilValue {
        match value {
            ZilValue::Nil => ZilValue::Nil,
            ZilValue::Number(n) => ZilValue::Number(*n),
            ZilValue::String(s) => ZilValue::String(s.clone()),
            ZilValue::Symbol(s) => {
                // Look up as object first
                if let Some(id) = self.get_object_id(s) {
                    return ZilValue::Number(id as i64);
                }
                // Then as global
                self.get_global(s)
            }
            ZilValue::LocalVar(name) => self.get_local(name),
            ZilValue::GlobalVar(name) => self.get_global(name),
            ZilValue::Form(items) => self.eval_form(items),
            ZilValue::List(items) => {
                let evaled: Vec<ZilValue> = items.iter().map(|i| self.eval(i)).collect();
                ZilValue::List(evaled)
            }
            ZilValue::Quote(inner) => (**inner).clone(),
            _ => value.clone(),
        }
    }

    fn eval_form(&mut self, items: &[ZilValue]) -> ZilValue {
        if items.is_empty() {
            return ZilValue::Nil;
        }

        let func_name = match &items[0] {
            ZilValue::Symbol(s) => s.clone(),
            ZilValue::Atom(s) => s.clone(),
            _ => return ZilValue::Nil,
        };

        let args = &items[1..];

        // Handle built-in forms
        match func_name.as_str() {
            // Arithmetic
            "+" => self.builtin_add(args),
            "-" => self.builtin_sub(args),
            "*" => self.builtin_mul(args),
            "/" => self.builtin_div(args),
            "MOD" => self.builtin_mod(args),
            "RANDOM" => self.builtin_random(args),
            
            // Comparison
            "==" | "EQUAL?" => self.builtin_equal(args),
            "==?" => self.builtin_equal(args),
            "N==?" => self.builtin_not_equal(args),
            "G?" | ">" => self.builtin_greater(args),
            "L?" | "<" => self.builtin_less(args),
            "G=?" | ">=" => self.builtin_greater_eq(args),
            "L=?" | "<=" => self.builtin_less_eq(args),
            "0?" | "ZERO?" => self.builtin_zero(args),
            "1?" => self.builtin_one(args),
            
            // Logic
            "NOT" => self.builtin_not(args),
            "AND" => self.builtin_and(args),
            "OR" => self.builtin_or(args),
            
            // Control flow
            "COND" => self.builtin_cond(args),
            "REPEAT" => self.builtin_repeat(args),
            "RETURN" => self.builtin_return(args),
            "PROG" => self.builtin_prog(args),
            "AGAIN" => self.builtin_again(),
            
            // Variables
            "SET" => self.builtin_set(args),
            "SETG" => self.builtin_setg(args),
            "GASSIGNED?" => self.builtin_gassigned(args),
            
            // Objects
            "IN?" => self.builtin_in(args),
            "LOC" => self.builtin_loc(args),
            "FIRST?" => self.builtin_first(args),
            "NEXT?" => self.builtin_next(args),
            "MOVE" => self.builtin_move(args),
            "REMOVE" => self.builtin_remove(args),
            "FSET?" => self.builtin_fset_p(args),
            "FSET" => self.builtin_fset(args),
            "FCLEAR" => self.builtin_fclear(args),
            "GETP" => self.builtin_getp(args),
            "PUTP" => self.builtin_putp(args),
            "GETB" => self.builtin_getb(args),
            "GET" => self.builtin_get(args),
            "PUT" => self.builtin_put(args),
            
            // I/O
            "TELL" => self.builtin_tell(args),
            "PRINT" => self.builtin_print(args),
            "PRINTN" => self.builtin_printn(args),
            "PRINTI" => self.builtin_printi(args),
            "PRINTD" => self.builtin_printd(args),
            "PRINTB" => self.builtin_printb(args),
            "PRINTC" => self.builtin_printc(args),
            "CRLF" => self.builtin_crlf(),
            "READ" => self.builtin_read(args),
            
            // Game specific
            "V-VERSION" => {
                println!("ZORK I: The Great Underground Empire");
                println!("Infocom interactive fiction - a fantasy story");
                println!("Copyright (c) 1981, 1982, 1983 Infocom, Inc.");
                println!("All rights reserved.");
                println!("ZORK is a registered trademark of Infocom, Inc.");
                println!("Release 119 / Serial number 880429");
                ZilValue::Number(1)
            }
            
            "RTRUE" => ZilValue::Number(1),
            "RFALSE" => ZilValue::Nil,
            "RFATAL" => ZilValue::Number(2),
            
            "QUIT" => {
                self.quit_flag = true;
                ZilValue::Nil
            }
            
            "RESTART" => {
                println!("Restarting...");
                ZilValue::Nil
            }
            
            "SAVE" => {
                println!("Save not implemented.");
                ZilValue::Nil
            }
            
            "RESTORE" => {
                println!("Restore not implemented.");
                ZilValue::Nil
            }
            
            "APPLY" => self.builtin_apply(args),
            
            // Default: try calling as routine
            _ => self.call_routine(&func_name, args),
        }
    }

    // Arithmetic operations
    fn builtin_add(&mut self, args: &[ZilValue]) -> ZilValue {
        let mut sum: i64 = 0;
        for arg in args {
            if let ZilValue::Number(n) = self.eval(arg) {
                sum += n;
            }
        }
        ZilValue::Number(sum)
    }

    fn builtin_sub(&mut self, args: &[ZilValue]) -> ZilValue {
        if args.is_empty() {
            return ZilValue::Number(0);
        }
        let first = match self.eval(&args[0]) {
            ZilValue::Number(n) => n,
            _ => 0,
        };
        if args.len() == 1 {
            return ZilValue::Number(-first);
        }
        let mut result = first;
        for arg in &args[1..] {
            if let ZilValue::Number(n) = self.eval(arg) {
                result -= n;
            }
        }
        ZilValue::Number(result)
    }

    fn builtin_mul(&mut self, args: &[ZilValue]) -> ZilValue {
        let mut product: i64 = 1;
        for arg in args {
            if let ZilValue::Number(n) = self.eval(arg) {
                product *= n;
            }
        }
        ZilValue::Number(product)
    }

    fn builtin_div(&mut self, args: &[ZilValue]) -> ZilValue {
        if args.len() < 2 {
            return ZilValue::Number(0);
        }
        let a = match self.eval(&args[0]) {
            ZilValue::Number(n) => n,
            _ => 0,
        };
        let b = match self.eval(&args[1]) {
            ZilValue::Number(n) => n,
            _ => 1,
        };
        if b == 0 {
            return ZilValue::Number(0);
        }
        ZilValue::Number(a / b)
    }

    fn builtin_mod(&mut self, args: &[ZilValue]) -> ZilValue {
        if args.len() < 2 {
            return ZilValue::Number(0);
        }
        let a = match self.eval(&args[0]) {
            ZilValue::Number(n) => n,
            _ => 0,
        };
        let b = match self.eval(&args[1]) {
            ZilValue::Number(n) => n,
            _ => 1,
        };
        if b == 0 {
            return ZilValue::Number(0);
        }
        ZilValue::Number(a % b)
    }

    fn builtin_random(&mut self, args: &[ZilValue]) -> ZilValue {
        let max = match self.eval(&args[0]) {
            ZilValue::Number(n) if n > 0 => n,
            _ => 100,
        };
        ZilValue::Number(self.random(max))
    }

    // Comparison operations
    fn builtin_equal(&mut self, args: &[ZilValue]) -> ZilValue {
        if args.len() < 2 {
            return ZilValue::Nil;
        }
        let first = self.eval(&args[0]);
        for arg in &args[1..] {
            let val = self.eval(arg);
            if self.values_equal(&first, &val) {
                return ZilValue::Number(1);
            }
        }
        ZilValue::Nil
    }

    fn builtin_not_equal(&mut self, args: &[ZilValue]) -> ZilValue {
        match self.builtin_equal(args) {
            ZilValue::Nil => ZilValue::Number(1),
            _ => ZilValue::Nil,
        }
    }

    fn values_equal(&self, a: &ZilValue, b: &ZilValue) -> bool {
        match (a, b) {
            (ZilValue::Nil, ZilValue::Nil) => true,
            (ZilValue::Number(na), ZilValue::Number(nb)) => na == nb,
            (ZilValue::String(sa), ZilValue::String(sb)) => sa == sb,
            (ZilValue::Symbol(sa), ZilValue::Symbol(sb)) => sa == sb,
            _ => false,
        }
    }

    fn builtin_greater(&mut self, args: &[ZilValue]) -> ZilValue {
        if args.len() < 2 {
            return ZilValue::Nil;
        }
        let a = match self.eval(&args[0]) {
            ZilValue::Number(n) => n,
            _ => 0,
        };
        let b = match self.eval(&args[1]) {
            ZilValue::Number(n) => n,
            _ => 0,
        };
        if a > b { ZilValue::Number(1) } else { ZilValue::Nil }
    }

    fn builtin_less(&mut self, args: &[ZilValue]) -> ZilValue {
        if args.len() < 2 {
            return ZilValue::Nil;
        }
        let a = match self.eval(&args[0]) {
            ZilValue::Number(n) => n,
            _ => 0,
        };
        let b = match self.eval(&args[1]) {
            ZilValue::Number(n) => n,
            _ => 0,
        };
        if a < b { ZilValue::Number(1) } else { ZilValue::Nil }
    }

    fn builtin_greater_eq(&mut self, args: &[ZilValue]) -> ZilValue {
        if args.len() < 2 {
            return ZilValue::Nil;
        }
        let a = match self.eval(&args[0]) {
            ZilValue::Number(n) => n,
            _ => 0,
        };
        let b = match self.eval(&args[1]) {
            ZilValue::Number(n) => n,
            _ => 0,
        };
        if a >= b { ZilValue::Number(1) } else { ZilValue::Nil }
    }

    fn builtin_less_eq(&mut self, args: &[ZilValue]) -> ZilValue {
        if args.len() < 2 {
            return ZilValue::Nil;
        }
        let a = match self.eval(&args[0]) {
            ZilValue::Number(n) => n,
            _ => 0,
        };
        let b = match self.eval(&args[1]) {
            ZilValue::Number(n) => n,
            _ => 0,
        };
        if a <= b { ZilValue::Number(1) } else { ZilValue::Nil }
    }

    fn builtin_zero(&mut self, args: &[ZilValue]) -> ZilValue {
        if args.is_empty() {
            return ZilValue::Nil;
        }
        match self.eval(&args[0]) {
            ZilValue::Number(0) => ZilValue::Number(1),
            ZilValue::Nil => ZilValue::Number(1),
            _ => ZilValue::Nil,
        }
    }

    fn builtin_one(&mut self, args: &[ZilValue]) -> ZilValue {
        if args.is_empty() {
            return ZilValue::Nil;
        }
        match self.eval(&args[0]) {
            ZilValue::Number(1) => ZilValue::Number(1),
            _ => ZilValue::Nil,
        }
    }

    // Logic operations
    fn builtin_not(&mut self, args: &[ZilValue]) -> ZilValue {
        if args.is_empty() {
            return ZilValue::Number(1);
        }
        if self.eval(&args[0]).to_bool() {
            ZilValue::Nil
        } else {
            ZilValue::Number(1)
        }
    }

    fn builtin_and(&mut self, args: &[ZilValue]) -> ZilValue {
        let mut result = ZilValue::Number(1);
        for arg in args {
            result = self.eval(arg);
            if !result.to_bool() {
                return ZilValue::Nil;
            }
        }
        result
    }

    fn builtin_or(&mut self, args: &[ZilValue]) -> ZilValue {
        for arg in args {
            let result = self.eval(arg);
            if result.to_bool() {
                return result;
            }
        }
        ZilValue::Nil
    }

    // Control flow
    fn builtin_cond(&mut self, args: &[ZilValue]) -> ZilValue {
        for arg in args {
            if let ZilValue::List(clause) | ZilValue::Form(clause) = arg {
                if clause.is_empty() {
                    continue;
                }
                // Check if condition is T or ELSE
                let condition = match &clause[0] {
                    ZilValue::Symbol(s) if s == "T" || s == "ELSE" => true,
                    _ => self.eval(&clause[0]).to_bool(),
                };
                
                if condition {
                    let mut result = ZilValue::Number(1);
                    for expr in &clause[1..] {
                        result = self.eval(expr);
                    }
                    return result;
                }
            }
        }
        ZilValue::Nil
    }

    fn builtin_repeat(&mut self, args: &[ZilValue]) -> ZilValue {
        if args.is_empty() {
            return ZilValue::Nil;
        }
        
        // First arg should be () for REPEAT
        let body_start = if matches!(&args[0], ZilValue::List(l) if l.is_empty()) {
            1
        } else {
            0
        };
        
        let mut result = ZilValue::Nil;
        for _ in 0..10000 {  // Prevent infinite loops
            for expr in &args[body_start..] {
                result = self.eval(expr);
                // Check for RETURN
                if let ZilValue::Form(items) = expr {
                    if !items.is_empty() {
                        if let ZilValue::Symbol(s) = &items[0] {
                            if s == "RETURN" {
                                if items.len() > 1 {
                                    return self.eval(&items[1]);
                                }
                                return ZilValue::Nil;
                            }
                        }
                    }
                }
            }
        }
        result
    }

    fn builtin_return(&mut self, args: &[ZilValue]) -> ZilValue {
        if args.is_empty() {
            return ZilValue::Nil;
        }
        self.eval(&args[0])
    }

    fn builtin_prog(&mut self, args: &[ZilValue]) -> ZilValue {
        if args.is_empty() {
            return ZilValue::Nil;
        }
        
        // First arg is local bindings, rest is body
        let mut result = ZilValue::Nil;
        for expr in &args[1..] {
            result = self.eval(expr);
        }
        result
    }

    fn builtin_again(&mut self) -> ZilValue {
        // In a full implementation, this would jump back
        ZilValue::Nil
    }

    // Variable operations
    fn builtin_set(&mut self, args: &[ZilValue]) -> ZilValue {
        if args.len() < 2 {
            return ZilValue::Nil;
        }
        let name = match &args[0] {
            ZilValue::Symbol(s) => s.clone(),
            ZilValue::Atom(s) => s.clone(),
            _ => return ZilValue::Nil,
        };
        let value = self.eval(&args[1]);
        self.set_local(&name, value.clone());
        value
    }

    fn builtin_setg(&mut self, args: &[ZilValue]) -> ZilValue {
        if args.len() < 2 {
            return ZilValue::Nil;
        }
        let name = match &args[0] {
            ZilValue::Symbol(s) => s.clone(),
            ZilValue::Atom(s) => s.clone(),
            _ => return ZilValue::Nil,
        };
        let value = self.eval(&args[1]);
        self.set_global(&name, value.clone());
        value
    }

    fn builtin_gassigned(&mut self, args: &[ZilValue]) -> ZilValue {
        if args.is_empty() {
            return ZilValue::Nil;
        }
        let name = match &args[0] {
            ZilValue::Symbol(s) => s,
            _ => return ZilValue::Nil,
        };
        if self.globals.contains_key(&name.to_uppercase()) {
            ZilValue::Number(1)
        } else {
            ZilValue::Nil
        }
    }

    // Object operations
    fn builtin_in(&mut self, args: &[ZilValue]) -> ZilValue {
        if args.len() < 2 {
            return ZilValue::Nil;
        }
        let obj_id = match self.eval(&args[0]) {
            ZilValue::Number(n) => n as usize,
            _ => return ZilValue::Nil,
        };
        let container_id = match self.eval(&args[1]) {
            ZilValue::Number(n) => n as usize,
            _ => return ZilValue::Nil,
        };
        if self.object_tree.in_(obj_id, container_id) {
            ZilValue::Number(1)
        } else {
            ZilValue::Nil
        }
    }

    fn builtin_loc(&mut self, args: &[ZilValue]) -> ZilValue {
        if args.is_empty() {
            return ZilValue::Nil;
        }
        let obj_id = match self.eval(&args[0]) {
            ZilValue::Number(n) => n as usize,
            _ => return ZilValue::Nil,
        };
        match self.object_tree.get_parent(obj_id) {
            Some(parent) => ZilValue::Number(parent as i64),
            None => ZilValue::Nil,
        }
    }

    fn builtin_first(&mut self, args: &[ZilValue]) -> ZilValue {
        if args.is_empty() {
            return ZilValue::Nil;
        }
        let obj_id = match self.eval(&args[0]) {
            ZilValue::Number(n) => n as usize,
            _ => return ZilValue::Nil,
        };
        match self.object_tree.get_first_child(obj_id) {
            Some(child) => ZilValue::Number(child as i64),
            None => ZilValue::Nil,
        }
    }

    fn builtin_next(&mut self, args: &[ZilValue]) -> ZilValue {
        if args.is_empty() {
            return ZilValue::Nil;
        }
        let obj_id = match self.eval(&args[0]) {
            ZilValue::Number(n) => n as usize,
            _ => return ZilValue::Nil,
        };
        match self.object_tree.get_sibling(obj_id) {
            Some(sibling) => ZilValue::Number(sibling as i64),
            None => ZilValue::Nil,
        }
    }

    fn builtin_move(&mut self, args: &[ZilValue]) -> ZilValue {
        if args.len() < 2 {
            return ZilValue::Nil;
        }
        let obj_id = match self.eval(&args[0]) {
            ZilValue::Number(n) => n as usize,
            _ => return ZilValue::Nil,
        };
        let dest_id = match self.eval(&args[1]) {
            ZilValue::Number(n) => n as usize,
            _ => return ZilValue::Nil,
        };
        self.move_object(obj_id, dest_id);
        ZilValue::Number(1)
    }

    fn builtin_remove(&mut self, args: &[ZilValue]) -> ZilValue {
        if args.is_empty() {
            return ZilValue::Nil;
        }
        let obj_id = match self.eval(&args[0]) {
            ZilValue::Number(n) => n as usize,
            _ => return ZilValue::Nil,
        };
        self.object_tree.set_parent(obj_id, None);
        ZilValue::Number(1)
    }

    fn builtin_fset_p(&mut self, args: &[ZilValue]) -> ZilValue {
        if args.len() < 2 {
            return ZilValue::Nil;
        }
        let obj_id = match self.eval(&args[0]) {
            ZilValue::Number(n) => n as usize,
            _ => return ZilValue::Nil,
        };
        let flag = match &args[1] {
            ZilValue::Symbol(s) => s.clone(),
            ZilValue::GlobalVar(s) => s.clone(),
            _ => return ZilValue::Nil,
        };
        if self.has_flag(obj_id, &flag) {
            ZilValue::Number(1)
        } else {
            ZilValue::Nil
        }
    }

    fn builtin_fset(&mut self, args: &[ZilValue]) -> ZilValue {
        if args.len() < 2 {
            return ZilValue::Nil;
        }
        let obj_id = match self.eval(&args[0]) {
            ZilValue::Number(n) => n as usize,
            _ => return ZilValue::Nil,
        };
        let flag = match &args[1] {
            ZilValue::Symbol(s) => s.clone(),
            ZilValue::GlobalVar(s) => s.clone(),
            _ => return ZilValue::Nil,
        };
        self.set_flag(obj_id, &flag);
        ZilValue::Number(1)
    }

    fn builtin_fclear(&mut self, args: &[ZilValue]) -> ZilValue {
        if args.len() < 2 {
            return ZilValue::Nil;
        }
        let obj_id = match self.eval(&args[0]) {
            ZilValue::Number(n) => n as usize,
            _ => return ZilValue::Nil,
        };
        let flag = match &args[1] {
            ZilValue::Symbol(s) => s.clone(),
            ZilValue::GlobalVar(s) => s.clone(),
            _ => return ZilValue::Nil,
        };
        self.clear_flag(obj_id, &flag);
        ZilValue::Number(1)
    }

    fn builtin_getp(&mut self, args: &[ZilValue]) -> ZilValue {
        if args.len() < 2 {
            return ZilValue::Nil;
        }
        let obj_id = match self.eval(&args[0]) {
            ZilValue::Number(n) => n as usize,
            _ => return ZilValue::Nil,
        };
        let prop = match &args[1] {
            ZilValue::Symbol(s) => s.clone(),
            ZilValue::GlobalVar(s) => s.clone(),
            _ => return ZilValue::Nil,
        };
        self.get_property(obj_id, &prop)
    }

    fn builtin_putp(&mut self, args: &[ZilValue]) -> ZilValue {
        if args.len() < 3 {
            return ZilValue::Nil;
        }
        let obj_id = match self.eval(&args[0]) {
            ZilValue::Number(n) => n as usize,
            _ => return ZilValue::Nil,
        };
        let prop = match &args[1] {
            ZilValue::Symbol(s) => s.clone(),
            ZilValue::GlobalVar(s) => s.clone(),
            _ => return ZilValue::Nil,
        };
        let value = self.eval(&args[2]);
        self.set_property(obj_id, &prop, value);
        ZilValue::Number(1)
    }

    fn builtin_getb(&mut self, args: &[ZilValue]) -> ZilValue {
        // Get byte from table
        self.builtin_get(args)
    }

    fn builtin_get(&mut self, args: &[ZilValue]) -> ZilValue {
        if args.len() < 2 {
            return ZilValue::Nil;
        }
        let table = self.eval(&args[0]);
        let index = match self.eval(&args[1]) {
            ZilValue::Number(n) => n as usize,
            _ => return ZilValue::Nil,
        };
        match table {
            ZilValue::List(items) | ZilValue::Table(items) => {
                items.get(index).cloned().unwrap_or(ZilValue::Nil)
            }
            _ => ZilValue::Nil,
        }
    }

    fn builtin_put(&mut self, args: &[ZilValue]) -> ZilValue {
        // Put into table - simplified implementation
        ZilValue::Number(1)
    }

    // I/O operations
    fn builtin_tell(&mut self, args: &[ZilValue]) -> ZilValue {
        for arg in args {
            match arg {
                ZilValue::String(s) => print!("{}", s),
                ZilValue::Symbol(s) if s == "CR" || s == "CRLF" => println!(),
                ZilValue::Symbol(s) if s == "D" || s == "DESC" => {
                    // Next arg should be object to describe
                }
                ZilValue::Symbol(s) if s == "N" || s == "NUM" => {
                    // Next arg should be number
                }
                ZilValue::Form(items) => {
                    if let Some(ZilValue::Symbol(s)) = items.first() {
                        if s == "GETP" || s == "PRINTD" || s == "PRINTI" {
                            let result = self.eval(arg);
                            match result {
                                ZilValue::String(s) => print!("{}", s),
                                ZilValue::Number(n) => print!("{}", n),
                                _ => {}
                            }
                        }
                    }
                }
                _ => {
                    let val = self.eval(arg);
                    match val {
                        ZilValue::String(s) => print!("{}", s),
                        ZilValue::Number(n) => print!("{}", n),
                        _ => {}
                    }
                }
            }
        }
        let _ = io::stdout().flush();
        ZilValue::Number(1)
    }

    fn builtin_print(&mut self, args: &[ZilValue]) -> ZilValue {
        if args.is_empty() {
            return ZilValue::Nil;
        }
        let val = self.eval(&args[0]);
        match val {
            ZilValue::String(s) => print!("{}", s),
            ZilValue::Number(n) => print!("{}", n),
            _ => print!("{}", val),
        }
        let _ = io::stdout().flush();
        ZilValue::Number(1)
    }

    fn builtin_printn(&mut self, args: &[ZilValue]) -> ZilValue {
        if args.is_empty() {
            return ZilValue::Nil;
        }
        let val = self.eval(&args[0]);
        if let ZilValue::Number(n) = val {
            print!("{}", n);
        }
        let _ = io::stdout().flush();
        ZilValue::Number(1)
    }

    fn builtin_printi(&mut self, args: &[ZilValue]) -> ZilValue {
        if args.is_empty() {
            return ZilValue::Nil;
        }
        match &args[0] {
            ZilValue::String(s) => print!("{}", s),
            _ => {}
        }
        let _ = io::stdout().flush();
        ZilValue::Number(1)
    }

    fn builtin_printd(&mut self, args: &[ZilValue]) -> ZilValue {
        if args.is_empty() {
            return ZilValue::Nil;
        }
        let obj_id = match self.eval(&args[0]) {
            ZilValue::Number(n) => n as usize,
            _ => return ZilValue::Nil,
        };
        if let Some(obj) = self.objects.get(obj_id) {
            if let Some(ZilValue::String(desc)) = obj.properties.get("DESC") {
                print!("{}", desc);
            } else {
                print!("{}", obj.name.to_lowercase());
            }
        }
        let _ = io::stdout().flush();
        ZilValue::Number(1)
    }

    fn builtin_printb(&mut self, args: &[ZilValue]) -> ZilValue {
        // Print string from byte address
        self.builtin_print(args)
    }

    fn builtin_printc(&mut self, args: &[ZilValue]) -> ZilValue {
        if args.is_empty() {
            return ZilValue::Nil;
        }
        let val = self.eval(&args[0]);
        if let ZilValue::Number(n) = val {
            print!("{}", char::from_u32(n as u32).unwrap_or('?'));
        }
        let _ = io::stdout().flush();
        ZilValue::Number(1)
    }

    fn builtin_crlf(&mut self) -> ZilValue {
        println!();
        ZilValue::Number(1)
    }

    fn builtin_read(&mut self, _args: &[ZilValue]) -> ZilValue {
        let _ = io::stdout().flush();
        let mut input = String::new();
        let _ = io::stdin().lock().read_line(&mut input);
        ZilValue::String(input.trim().to_string())
    }

    fn builtin_apply(&mut self, args: &[ZilValue]) -> ZilValue {
        if args.is_empty() {
            return ZilValue::Nil;
        }
        let func = self.eval(&args[0]);
        match func {
            ZilValue::Symbol(name) => {
                self.call_routine(&name, &args[1..])
            }
            ZilValue::Nil => ZilValue::Nil,
            ZilValue::Number(0) => ZilValue::Nil,
            _ => ZilValue::Nil,
        }
    }

    fn call_routine(&mut self, name: &str, args: &[ZilValue]) -> ZilValue {
        let routine = match self.routines.get(&name.to_uppercase()) {
            Some(r) => r.clone(),
            None => return ZilValue::Nil,
        };

        // Create new call frame
        let mut frame = HashMap::new();

        // Bind arguments
        for (i, (arg_name, _is_optional)) in routine.args.iter().enumerate() {
            let value = if i < args.len() {
                self.eval(&args[i])
            } else {
                ZilValue::Nil
            };
            frame.insert(arg_name.clone(), value);
        }

        // Initialize aux variables
        for (name, default) in &routine.aux_vars {
            let value = default.clone().unwrap_or(ZilValue::Nil);
            frame.insert(name.clone(), value);
        }

        self.call_stack.push(frame);

        // Execute body
        let mut result = ZilValue::Nil;
        for expr in &routine.body {
            result = self.eval(expr);
            // Check for early return (RTRUE, RFALSE)
            if let ZilValue::Form(items) = expr {
                if let Some(ZilValue::Symbol(s)) = items.first() {
                    if s == "RTRUE" || s == "RFALSE" || s == "RETURN" || s == "RFATAL" {
                        break;
                    }
                }
            }
        }

        self.call_stack.pop();
        result
    }
}
