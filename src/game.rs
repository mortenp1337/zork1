/// ZIL Game Engine - Runs the Zork game
use crate::runtime::Runtime;
use crate::ast::ZilValue;
use std::io::{self, Write, BufRead};
use std::collections::HashSet;

pub struct GameEngine {
    runtime: Runtime,
    current_room: usize,
    player: usize,
    verbose: bool,
    visited_rooms: HashSet<usize>,
}

impl GameEngine {
    pub fn new(runtime: Runtime) -> Self {
        GameEngine {
            runtime,
            current_room: 0,
            player: 0,
            verbose: true,
            visited_rooms: HashSet::new(),
        }
    }

    pub fn init(&mut self) {
        // Find initial room (WEST-OF-HOUSE)
        if let Some(id) = self.runtime.get_object_id("WEST-OF-HOUSE") {
            self.current_room = id;
            self.runtime.set_global("HERE", ZilValue::Number(id as i64));
        }

        // Find or create player/adventurer
        if let Some(id) = self.runtime.get_object_id("ADVENTURER") {
            self.player = id;
            self.runtime.set_global("WINNER", ZilValue::Number(id as i64));
            self.runtime.set_global("PLAYER", ZilValue::Number(id as i64));
            // Move player to current room
            self.runtime.move_object(id, self.current_room);
        }

        // Initialize LIT
        self.runtime.set_global("LIT", ZilValue::Number(1));
        self.runtime.lit = true;
    }

    pub fn run(&mut self) {
        self.print_intro();
        self.look();

        loop {
            if self.runtime.quit_flag || self.runtime.dead {
                break;
            }

            print!("\n>");
            let _ = io::stdout().flush();

            let mut input = String::new();
            match io::stdin().lock().read_line(&mut input) {
                Ok(0) => break, // EOF
                Ok(_) => {
                    let input = input.trim().to_uppercase();
                    if input.is_empty() {
                        continue;
                    }
                    self.process_command(&input);
                    self.runtime.moves += 1;
                }
                Err(_) => break,
            }
        }

        println!("\nThank you for playing Zork I!");
    }

    fn print_intro(&self) {
        println!("ZORK I: The Great Underground Empire");
        println!("Copyright (c) 1981, 1982, 1983 Infocom, Inc. All rights reserved.");
        println!("ZORK is a registered trademark of Infocom, Inc.");
        println!("Release 119 / Serial number 880429 / Rust Edition");
        println!();
    }

    fn look(&mut self) {
        if let Some(room) = self.runtime.get_object(self.current_room) {
            // Print room name
            if let Some(ZilValue::String(desc)) = room.properties.get("DESC") {
                println!("{}", desc);
            } else {
                println!("{}", room.name);
            }

            // Print room description
            let first_visit = !self.visited_rooms.contains(&self.current_room);
            if first_visit || self.verbose {
                if let Some(ZilValue::String(ldesc)) = room.properties.get("LDESC") {
                    println!("{}", ldesc);
                }
            }

            self.visited_rooms.insert(self.current_room);
        }

        // List objects in room
        self.list_room_contents();
    }

    fn list_room_contents(&self) {
        let mut found_objects = false;

        // Iterate through objects to find those in current room
        for id in 0..self.runtime.objects.len() {
            if id == self.player {
                continue;
            }

            if self.runtime.object_tree.in_(id, self.current_room) {
                // Check if object is visible
                if self.runtime.has_flag(id, "INVISIBLE") || self.runtime.has_flag(id, "NDESCBIT") {
                    continue;
                }

                if let Some(obj) = self.runtime.get_object(id) {
                    if !found_objects {
                        found_objects = true;
                    }

                    // Print object description
                    if let Some(ZilValue::String(fdesc)) = obj.properties.get("FDESC") {
                        println!("{}", fdesc);
                    } else if let Some(ZilValue::String(ldesc)) = obj.properties.get("LDESC") {
                        println!("{}", ldesc);
                    } else if let Some(ZilValue::String(desc)) = obj.properties.get("DESC") {
                        println!("There is a {} here.", desc);
                    }
                }
            }
        }
    }

    fn process_command(&mut self, input: &str) {
        let words: Vec<&str> = input.split_whitespace().collect();
        if words.is_empty() {
            return;
        }

        let verb = self.resolve_synonym(words[0]);
        
        match verb.as_str() {
            // Movement
            "N" | "NORTH" => self.go_direction("NORTH"),
            "S" | "SOUTH" => self.go_direction("SOUTH"),
            "E" | "EAST" => self.go_direction("EAST"),
            "W" | "WEST" => self.go_direction("WEST"),
            "NE" | "NORTHEAST" | "NORTHE" => self.go_direction("NE"),
            "NW" | "NORTHWEST" => self.go_direction("NW"),
            "SE" | "SOUTHEAST" | "SOUTHE" => self.go_direction("SE"),
            "SW" | "SOUTHWEST" => self.go_direction("SW"),
            "U" | "UP" => self.go_direction("UP"),
            "D" | "DOWN" => self.go_direction("DOWN"),
            "IN" | "ENTER" => self.go_direction("IN"),
            "OUT" | "EXIT" | "LEAVE" => self.go_direction("OUT"),
            
            // Look
            "L" | "LOOK" => {
                if words.len() > 1 {
                    match words[1] {
                        "AT" if words.len() > 2 => self.examine(&words[2..].join(" ")),
                        "IN" | "INSIDE" if words.len() > 2 => self.look_in(&words[2..].join(" ")),
                        _ => self.examine(&words[1..].join(" ")),
                    }
                } else {
                    self.verbose = true;
                    self.look();
                }
            }
            "EXAMINE" | "X" => {
                if words.len() > 1 {
                    self.examine(&words[1..].join(" "));
                } else {
                    println!("What do you want to examine?");
                }
            }
            
            // Inventory
            "I" | "INVENTORY" => self.inventory(),
            
            // Take/Get
            "TAKE" | "GET" | "PICK" | "GRAB" => {
                if words.len() > 1 {
                    let obj_name = if words[1] == "UP" && words.len() > 2 {
                        words[2..].join(" ")
                    } else {
                        words[1..].join(" ")
                    };
                    self.take(&obj_name);
                } else {
                    println!("What do you want to take?");
                }
            }
            
            // Drop
            "DROP" | "PUT" => {
                if words.len() > 1 {
                    if words[1] == "DOWN" && words.len() > 2 {
                        self.drop(&words[2..].join(" "));
                    } else if words.len() > 2 && (words.iter().any(|&w| w == "IN" || w == "ON")) {
                        // Handle "put X in Y"
                        let in_pos = words.iter().position(|&w| w == "IN" || w == "ON");
                        if let Some(pos) = in_pos {
                            let obj = words[1..pos].join(" ");
                            let container = words[pos+1..].join(" ");
                            self.put_in(&obj, &container);
                        } else {
                            self.drop(&words[1..].join(" "));
                        }
                    } else {
                        self.drop(&words[1..].join(" "));
                    }
                } else {
                    println!("What do you want to drop?");
                }
            }
            
            // Open/Close
            "OPEN" => {
                if words.len() > 1 {
                    self.open(&words[1..].join(" "));
                } else {
                    println!("What do you want to open?");
                }
            }
            "CLOSE" | "SHUT" => {
                if words.len() > 1 {
                    self.close(&words[1..].join(" "));
                } else {
                    println!("What do you want to close?");
                }
            }
            
            // Read
            "READ" => {
                if words.len() > 1 {
                    self.read(&words[1..].join(" "));
                } else {
                    println!("What do you want to read?");
                }
            }
            
            // Game commands
            "SCORE" => self.show_score(),
            "QUIT" | "Q" => {
                print!("Your score is {} (total of 350 points), in {} moves.\n", 
                       self.runtime.score, self.runtime.moves);
                print!("Do you wish to leave the game? (Y is affirmative): ");
                let _ = io::stdout().flush();
                let mut response = String::new();
                let _ = io::stdin().lock().read_line(&mut response);
                if response.trim().to_uppercase().starts_with('Y') {
                    self.runtime.quit_flag = true;
                }
            }
            "VERBOSE" => {
                self.verbose = true;
                println!("Maximum verbosity.");
            }
            "BRIEF" => {
                self.verbose = false;
                println!("Brief descriptions.");
            }
            "DIAGNOSE" => self.diagnose(),
            "SAVE" => println!("Save not implemented in this version."),
            "RESTORE" => println!("Restore not implemented in this version."),
            "RESTART" => {
                print!("Do you wish to restart? (Y is affirmative): ");
                let _ = io::stdout().flush();
                let mut response = String::new();
                let _ = io::stdin().lock().read_line(&mut response);
                if response.trim().to_uppercase().starts_with('Y') {
                    println!("Restarting...");
                    // In a full implementation, this would reset the game state
                }
            }
            "VERSION" => {
                println!("ZORK I: The Great Underground Empire");
                println!("Infocom interactive fiction - a fantasy story");
                println!("Copyright (c) 1981, 1982, 1983, 1984, 1985, 1986 Infocom, Inc.");
                println!("Release 119 / Serial number 880429 / Rust Edition");
            }
            "WAIT" | "Z" => {
                println!("Time passes...");
            }
            
            // Light
            "TURN" => {
                if words.len() > 2 && words[1] == "ON" {
                    self.turn_on(&words[2..].join(" "));
                } else if words.len() > 2 && words[1] == "OFF" {
                    self.turn_off(&words[2..].join(" "));
                } else {
                    println!("I don't understand that.");
                }
            }
            "LIGHT" | "ON" => {
                if words.len() > 1 {
                    self.turn_on(&words[1..].join(" "));
                } else {
                    println!("What do you want to turn on?");
                }
            }
            
            // Special
            "XYZZY" | "PLUGH" => {
                println!("A hollow voice says \"Fool.\"");
            }
            "HELLO" | "HI" => {
                if words.len() > 1 && words[1] == "SAILOR" {
                    println!("Nothing happens here.");
                } else {
                    println!("Hello.");
                }
            }
            "PRAY" => {
                if let Some(room) = self.runtime.get_object(self.current_room) {
                    if room.name.contains("TEMPLE") || room.name.contains("ALTAR") {
                        println!("A voice booms out of the darkness.");
                    } else {
                        println!("If you pray often, I may be able to help you.");
                    }
                }
            }
            "JUMP" | "LEAP" => {
                println!("Wheeeeee!!!!!!");
            }
            "ECHO" => {
                println!("echo echo ...");
            }
            "CLIMB" => {
                if words.len() > 1 {
                    if words[1] == "UP" {
                        self.go_direction("UP");
                    } else if words[1] == "DOWN" {
                        self.go_direction("DOWN");
                    } else {
                        self.go_direction("UP");
                    }
                } else {
                    self.go_direction("UP");
                }
            }

            "GO" | "WALK" | "RUN" => {
                if words.len() > 1 {
                    let dir = self.resolve_synonym(words[1]);
                    self.go_direction(&dir);
                } else {
                    println!("Which way do you want to go?");
                }
            }

            _ => {
                println!("I don't understand that.");
            }
        }
    }

    fn resolve_synonym(&self, word: &str) -> String {
        let upper = word.to_uppercase();
        self.runtime.synonyms.get(&upper).cloned().unwrap_or(upper)
    }

    fn go_direction(&mut self, direction: &str) {
        if let Some(room) = self.runtime.get_object(self.current_room) {
            // Check for exit in that direction
            if let Some(exit) = room.properties.get(direction) {
                match exit {
                    ZilValue::Symbol(dest_name) => {
                        if let Some(dest_id) = self.runtime.get_object_id(dest_name) {
                            self.current_room = dest_id;
                            self.runtime.set_global("HERE", ZilValue::Number(dest_id as i64));
                            self.runtime.move_object(self.player, dest_id);
                            self.look();
                        } else {
                            println!("You can't go that way.");
                        }
                    }
                    ZilValue::String(msg) => {
                        println!("{}", msg);
                    }
                    ZilValue::Number(dest_id) => {
                        let dest = *dest_id as usize;
                        self.current_room = dest;
                        self.runtime.set_global("HERE", ZilValue::Number(*dest_id));
                        self.runtime.move_object(self.player, dest);
                        self.look();
                    }
                    ZilValue::Form(items) | ZilValue::List(items) => {
                        // Complex exit - could be conditional
                        // Try to find destination in the form
                        for item in items {
                            if let ZilValue::Symbol(s) = item {
                                if s == "TO" {
                                    continue;
                                }
                                if let Some(dest_id) = self.runtime.get_object_id(s) {
                                    self.current_room = dest_id;
                                    self.runtime.set_global("HERE", ZilValue::Number(dest_id as i64));
                                    self.runtime.move_object(self.player, dest_id);
                                    self.look();
                                    return;
                                }
                            }
                        }
                        println!("You can't go that way.");
                    }
                    _ => {
                        println!("You can't go that way.");
                    }
                }
            } else {
                println!("You can't go that way.");
            }
        }
    }

    fn examine(&mut self, obj_name: &str) {
        let obj_name = obj_name.to_uppercase();
        
        // First check room for matching object
        if let Some(id) = self.find_object_in_scope(&obj_name) {
            if let Some(obj) = self.runtime.get_object(id) {
                // Check for TEXT property first (for readable items)
                if let Some(ZilValue::String(text)) = obj.properties.get("TEXT") {
                    println!("{}", text);
                    return;
                }
                
                // Then LDESC
                if let Some(ZilValue::String(ldesc)) = obj.properties.get("LDESC") {
                    println!("{}", ldesc);
                    return;
                }
                
                // Then FDESC
                if let Some(ZilValue::String(fdesc)) = obj.properties.get("FDESC") {
                    println!("{}", fdesc);
                    return;
                }
                
                // Default description
                if let Some(ZilValue::String(desc)) = obj.properties.get("DESC") {
                    println!("You see nothing special about the {}.", desc);
                } else {
                    println!("You see nothing special about it.");
                }
                
                // If it's a container, list contents
                if self.runtime.has_flag(id, "CONTBIT") || self.runtime.has_flag(id, "OPENBIT") {
                    self.look_in(&obj_name);
                }
            }
        } else {
            println!("You can't see any {} here.", obj_name.to_lowercase());
        }
    }

    fn look_in(&self, obj_name: &str) {
        let obj_name = obj_name.to_uppercase();
        
        if let Some(container_id) = self.find_object_in_scope(&obj_name) {
            // Check if container is open or transparent
            let is_open = self.runtime.has_flag(container_id, "OPENBIT");
            let is_trans = self.runtime.has_flag(container_id, "TRANSBIT");
            
            if !is_open && !is_trans {
                println!("It's closed.");
                return;
            }
            
            let mut found = false;
            for id in 0..self.runtime.objects.len() {
                if self.runtime.object_tree.in_(id, container_id) {
                    if !found {
                        if let Some(container) = self.runtime.get_object(container_id) {
                            if let Some(ZilValue::String(desc)) = container.properties.get("DESC") {
                                println!("The {} contains:", desc);
                            }
                        }
                        found = true;
                    }
                    if let Some(obj) = self.runtime.get_object(id) {
                        if let Some(ZilValue::String(desc)) = obj.properties.get("DESC") {
                            println!("  A {}", desc);
                        }
                    }
                }
            }
            
            if !found {
                println!("It's empty.");
            }
        } else {
            println!("You can't see any {} here.", obj_name.to_lowercase());
        }
    }

    fn inventory(&self) {
        let mut found = false;
        
        for id in 0..self.runtime.objects.len() {
            if self.runtime.object_tree.in_(id, self.player) {
                if !found {
                    println!("You are carrying:");
                    found = true;
                }
                if let Some(obj) = self.runtime.get_object(id) {
                    if let Some(ZilValue::String(desc)) = obj.properties.get("DESC") {
                        println!("  A {}", desc);
                    }
                }
            }
        }
        
        if !found {
            println!("You are empty-handed.");
        }
    }

    fn find_object_in_scope(&self, name: &str) -> Option<usize> {
        let name = name.to_uppercase();
        let words: Vec<&str> = name.split_whitespace().collect();
        
        // Check room contents
        for id in 0..self.runtime.objects.len() {
            if self.runtime.object_tree.in_(id, self.current_room) 
                || self.runtime.object_tree.in_(id, self.player) {
                if self.object_matches(id, &words) {
                    return Some(id);
                }
            }
        }
        
        // Check by exact name
        self.runtime.get_object_id(&name)
    }

    fn object_matches(&self, id: usize, words: &[&str]) -> bool {
        if let Some(obj) = self.runtime.get_object(id) {
            // Check name
            for word in words {
                if obj.name.to_uppercase().contains(&word.to_uppercase()) {
                    return true;
                }
                
                // Check DESC
                if let Some(ZilValue::String(desc)) = obj.properties.get("DESC") {
                    if desc.to_uppercase().contains(&word.to_uppercase()) {
                        return true;
                    }
                }
                
                // Check SYNONYM
                if let Some(ZilValue::List(syns)) = obj.properties.get("SYNONYM") {
                    for syn in syns {
                        if let ZilValue::Symbol(s) = syn {
                            if s.to_uppercase() == word.to_uppercase() {
                                return true;
                            }
                        }
                    }
                }
            }
        }
        false
    }

    fn take(&mut self, obj_name: &str) {
        let obj_name = obj_name.to_uppercase();
        
        if let Some(id) = self.find_object_in_scope(&obj_name) {
            // Check if takeable
            if !self.runtime.has_flag(id, "TAKEBIT") {
                if self.runtime.has_flag(id, "TRYTAKEBIT") {
                    // Has special take action
                    println!("You can't take that.");
                } else {
                    println!("You can't take that.");
                }
                return;
            }
            
            // Check if already carrying
            if self.runtime.object_tree.in_(id, self.player) {
                println!("You're already carrying that!");
                return;
            }
            
            // Take it
            self.runtime.move_object(id, self.player);
            println!("Taken.");
        } else {
            println!("You can't see any {} here.", obj_name.to_lowercase());
        }
    }

    fn drop(&mut self, obj_name: &str) {
        let obj_name = obj_name.to_uppercase();
        
        // Find in inventory
        for id in 0..self.runtime.objects.len() {
            if self.runtime.object_tree.in_(id, self.player) {
                if self.object_matches(id, &obj_name.split_whitespace().collect::<Vec<_>>()) {
                    self.runtime.move_object(id, self.current_room);
                    println!("Dropped.");
                    return;
                }
            }
        }
        
        println!("You're not carrying any {}.", obj_name.to_lowercase());
    }

    fn put_in(&mut self, obj_name: &str, container_name: &str) {
        let obj_name = obj_name.to_uppercase();
        let container_name = container_name.to_uppercase();
        
        // Find object in inventory
        let mut obj_id = None;
        for id in 0..self.runtime.objects.len() {
            if self.runtime.object_tree.in_(id, self.player) {
                if self.object_matches(id, &obj_name.split_whitespace().collect::<Vec<_>>()) {
                    obj_id = Some(id);
                    break;
                }
            }
        }
        
        let obj_id = match obj_id {
            Some(id) => id,
            None => {
                println!("You're not carrying that.");
                return;
            }
        };
        
        // Find container
        if let Some(container_id) = self.find_object_in_scope(&container_name) {
            if !self.runtime.has_flag(container_id, "CONTBIT") {
                println!("That can't contain things.");
                return;
            }
            
            if !self.runtime.has_flag(container_id, "OPENBIT") {
                println!("It's closed.");
                return;
            }
            
            self.runtime.move_object(obj_id, container_id);
            println!("Done.");
        } else {
            println!("You can't see any {} here.", container_name.to_lowercase());
        }
    }

    fn open(&mut self, obj_name: &str) {
        let obj_name = obj_name.to_uppercase();
        
        if let Some(id) = self.find_object_in_scope(&obj_name) {
            if !self.runtime.has_flag(id, "CONTBIT") && !self.runtime.has_flag(id, "DOORBIT") {
                println!("You can't open that.");
                return;
            }
            
            if self.runtime.has_flag(id, "OPENBIT") {
                println!("It's already open.");
                return;
            }
            
            self.runtime.set_flag(id, "OPENBIT");
            println!("Opened.");
            
            // Show contents
            self.look_in(&obj_name);
        } else {
            println!("You can't see any {} here.", obj_name.to_lowercase());
        }
    }

    fn close(&mut self, obj_name: &str) {
        let obj_name = obj_name.to_uppercase();
        
        if let Some(id) = self.find_object_in_scope(&obj_name) {
            if !self.runtime.has_flag(id, "CONTBIT") && !self.runtime.has_flag(id, "DOORBIT") {
                println!("You can't close that.");
                return;
            }
            
            if !self.runtime.has_flag(id, "OPENBIT") {
                println!("It's already closed.");
                return;
            }
            
            self.runtime.clear_flag(id, "OPENBIT");
            println!("Closed.");
        } else {
            println!("You can't see any {} here.", obj_name.to_lowercase());
        }
    }

    fn read(&self, obj_name: &str) {
        let obj_name = obj_name.to_uppercase();
        
        if let Some(id) = self.find_object_in_scope(&obj_name) {
            if !self.runtime.has_flag(id, "READBIT") {
                println!("There's nothing written on it.");
                return;
            }
            
            if let Some(obj) = self.runtime.get_object(id) {
                if let Some(ZilValue::String(text)) = obj.properties.get("TEXT") {
                    println!("{}", text);
                } else {
                    println!("There's nothing written on it.");
                }
            }
        } else {
            println!("You can't see any {} here.", obj_name.to_lowercase());
        }
    }

    fn turn_on(&mut self, obj_name: &str) {
        let obj_name = obj_name.to_uppercase();
        
        if let Some(id) = self.find_object_in_scope(&obj_name) {
            if !self.runtime.has_flag(id, "LIGHTBIT") {
                println!("You can't turn that on.");
                return;
            }
            
            if self.runtime.has_flag(id, "ONBIT") {
                println!("It's already on.");
                return;
            }
            
            self.runtime.set_flag(id, "ONBIT");
            self.runtime.lit = true;
            self.runtime.set_global("LIT", ZilValue::Number(1));
            println!("The lamp is now on.");
        } else {
            println!("You can't see any {} here.", obj_name.to_lowercase());
        }
    }

    fn turn_off(&mut self, obj_name: &str) {
        let obj_name = obj_name.to_uppercase();
        
        if let Some(id) = self.find_object_in_scope(&obj_name) {
            if !self.runtime.has_flag(id, "ONBIT") {
                println!("It's not on.");
                return;
            }
            
            self.runtime.clear_flag(id, "ONBIT");
            println!("The lamp is now off.");
        } else {
            println!("You can't see any {} here.", obj_name.to_lowercase());
        }
    }

    fn show_score(&self) {
        println!("Your score is {} (total of 350 points), in {} move{}.",
                 self.runtime.score,
                 self.runtime.moves,
                 if self.runtime.moves == 1 { "" } else { "s" });
        
        let rank = match self.runtime.score {
            s if s >= 350 => "Master Adventurer",
            s if s >= 330 => "Winner",
            s if s >= 200 => "Adventurer",
            s if s >= 100 => "Junior Adventurer",
            s if s >= 50 => "Amateur Adventurer",
            s if s >= 25 => "Novice Adventurer",
            _ => "Beginner",
        };
        
        println!("This gives you the rank of {}.", rank);
    }

    fn diagnose(&self) {
        println!("You are in perfect health.");
    }
}
