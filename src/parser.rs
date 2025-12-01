/// ZIL Parser - Parses tokens into AST
use crate::lexer::{Lexer, Token};
use crate::ast::*;
use std::collections::HashMap;

pub struct Parser<'a> {
    lexer: Lexer<'a>,
    current_token: Token,
}

impl<'a> Parser<'a> {
    pub fn new(input: &'a str) -> Self {
        let mut lexer = Lexer::new(input);
        let current_token = lexer.next_token();
        Parser {
            lexer,
            current_token,
        }
    }

    fn advance(&mut self) {
        self.current_token = self.lexer.next_token();
    }

    fn expect(&mut self, expected: Token) -> Result<(), String> {
        if std::mem::discriminant(&self.current_token) == std::mem::discriminant(&expected) {
            self.advance();
            Ok(())
        } else {
            Err(format!(
                "Expected {:?}, got {:?} at line {}",
                expected, self.current_token, self.lexer.line
            ))
        }
    }

    pub fn parse(&mut self) -> Result<Vec<ZilDefinition>, String> {
        let mut definitions = Vec::new();
        let mut iteration = 0;
        const MAX_ITERATIONS: usize = 100000;

        loop {
            iteration += 1;
            if iteration > MAX_ITERATIONS {
                break; // Stop gracefully
            }
            
            match &self.current_token {
                Token::EOF => break,
                Token::LParen => {
                    match self.parse_top_level() {
                        Ok(Some(def)) => definitions.push(def),
                        Ok(None) => {}
                        Err(_) => {
                            // Try to recover by skipping to next top-level form
                            self.skip_form().ok();
                        }
                    }
                }
                Token::String(_) => {
                    // Top-level strings are comments, skip them
                    self.advance();
                }
                _ => {
                    self.advance(); // Skip unknown tokens at top level
                }
            }
        }

        Ok(definitions)
    }

    fn parse_top_level(&mut self) -> Result<Option<ZilDefinition>, String> {
        self.advance(); // consume <

        let symbol = match &self.current_token {
            Token::Symbol(s) => s.clone(),
            _ => {
                // Skip this form
                self.skip_form()?;
                return Ok(None);
            }
        };
        self.advance();

        let result = match symbol.as_str() {
            "OBJECT" => {
                let obj = self.parse_object()?;
                Some(ZilDefinition::Object(obj))
            }
            "ROOM" => {
                let obj = self.parse_object()?;
                Some(ZilDefinition::Room(obj))
            }
            "ROUTINE" => Some(ZilDefinition::Routine(self.parse_routine()?)),
            "DEFINE" => Some(ZilDefinition::Macro(self.parse_routine()?)),
            "DEFMAC" => Some(ZilDefinition::Macro(self.parse_defmac()?)),
            "SYNTAX" => Some(ZilDefinition::Syntax(self.parse_syntax()?)),
            "GLOBAL" => Some(ZilDefinition::Global(self.parse_global()?)),
            "CONSTANT" => Some(ZilDefinition::Constant(self.parse_constant()?)),
            "SYNONYM" => Some(self.parse_synonym()?),
            "BUZZ" => Some(self.parse_buzz()?),
            "DIRECTIONS" => Some(self.parse_directions()?),
            "PROPDEF" => Some(self.parse_propdef()?),
            "VERSION" => Some(self.parse_version()?),
            "INSERT-FILE" => Some(self.parse_insert_file()?),
            "SETG" | "SET" | "OR" | "AND" | "COND" | "PRINC" | "FREQUENT-WORDS?" => {
                // These are executable forms at top level, skip them
                self.skip_form()?;
                None
            }
            _ => {
                // Unknown form, skip it
                self.skip_form()?;
                None
            }
        };

        Ok(result)
    }

    fn skip_form(&mut self) -> Result<(), String> {
        let mut depth = 1;
        let mut iterations = 0;
        const MAX_ITERATIONS: usize = 100000;
        
        while depth > 0 {
            iterations += 1;
            if iterations > MAX_ITERATIONS {
                return Err(format!("Parser stuck at line {}", self.lexer.line));
            }
            
            match &self.current_token {
                Token::LParen | Token::LBracket => {
                    depth += 1;
                    self.advance();
                }
                Token::RParen | Token::RBracket => {
                    depth -= 1;
                    self.advance();
                }
                Token::EOF => return Err("Unexpected EOF while skipping form".to_string()),
                _ => self.advance(),
            }
        }
        Ok(())
    }

    fn parse_object(&mut self) -> Result<ZilObject, String> {
        let name = match &self.current_token {
            Token::Symbol(s) => s.clone(),
            _ => return Err(format!("Expected object name, got {:?}", self.current_token)),
        };
        self.advance();

        let mut obj = ZilObject::new(&name);

        // Parse properties
        while !matches!(self.current_token, Token::RParen | Token::EOF) {
            if let Token::LBracket = self.current_token {
                self.advance(); // consume (
                
                let prop_name = match &self.current_token {
                    Token::Symbol(s) => s.clone(),
                    _ => {
                        self.skip_to_close_bracket()?;
                        continue;
                    }
                };
                self.advance();

                match prop_name.as_str() {
                    "IN" | "LOC" => {
                        if let Token::Symbol(parent) = &self.current_token {
                            obj.parent = Some(parent.clone());
                        }
                        self.skip_to_close_bracket()?;
                    }
                    "FLAGS" => {
                        while !matches!(self.current_token, Token::RBracket | Token::EOF) {
                            if let Token::Symbol(flag) = &self.current_token {
                                obj.flags.push(flag.clone());
                            }
                            self.advance();
                        }
                        if let Token::RBracket = self.current_token {
                            self.advance();
                        }
                    }
                    _ => {
                        // Parse property value(s)
                        let mut values = Vec::new();
                        while !matches!(self.current_token, Token::RBracket | Token::EOF) {
                            values.push(self.parse_value()?);
                        }
                        let value = if values.len() == 1 {
                            values.pop().unwrap()
                        } else {
                            ZilValue::List(values)
                        };
                        obj.properties.insert(prop_name, value);
                        if let Token::RBracket = self.current_token {
                            self.advance();
                        }
                    }
                }
            } else {
                self.advance();
            }
        }

        if let Token::RParen = self.current_token {
            self.advance();
        }

        Ok(obj)
    }

    fn skip_to_close_bracket(&mut self) -> Result<(), String> {
        let mut depth = 1;
        while depth > 0 {
            match &self.current_token {
                Token::LBracket => {
                    depth += 1;
                    self.advance();
                }
                Token::RBracket => {
                    depth -= 1;
                    self.advance();
                }
                Token::EOF => return Err("Unexpected EOF".to_string()),
                _ => self.advance(),
            }
        }
        Ok(())
    }

    fn parse_routine(&mut self) -> Result<ZilRoutine, String> {
        let name = match &self.current_token {
            Token::Symbol(s) => s.clone(),
            _ => return Err(format!("Expected routine name, got {:?}", self.current_token)),
        };
        self.advance();

        let mut routine = ZilRoutine::new(&name);

        // Parse argument list
        if let Token::LBracket = self.current_token {
            self.advance();
            let mut in_optional = false;
            let mut in_aux = false;

            while !matches!(self.current_token, Token::RBracket | Token::EOF) {
                match &self.current_token {
                    Token::String(s) if s == "OPTIONAL" => {
                        in_optional = true;
                        self.advance();
                    }
                    Token::String(s) if s == "AUX" => {
                        in_aux = true;
                        self.advance();
                    }
                    Token::Symbol(s) => {
                        if in_aux {
                            routine.aux_vars.push((s.clone(), None));
                        } else {
                            routine.args.push((s.clone(), in_optional));
                        }
                        self.advance();
                    }
                    Token::LBracket => {
                        // Default value for aux var
                        self.advance();
                        if let Token::Symbol(var_name) = &self.current_token {
                            let var_name = var_name.clone();
                            self.advance();
                            let default_val = self.parse_value()?;
                            if in_aux {
                                routine.aux_vars.push((var_name, Some(default_val)));
                            } else {
                                routine.args.push((var_name, true));
                            }
                        }
                        self.skip_to_close_bracket()?;
                    }
                    _ => self.advance(),
                }
            }
            if let Token::RBracket = self.current_token {
                self.advance();
            }
        }

        // Parse body
        while !matches!(self.current_token, Token::RParen | Token::EOF) {
            routine.body.push(self.parse_value()?);
        }

        if let Token::RParen = self.current_token {
            self.advance();
        }

        Ok(routine)
    }

    fn parse_defmac(&mut self) -> Result<ZilRoutine, String> {
        // DEFMAC is similar to ROUTINE but for macros
        self.parse_routine()
    }

    fn parse_syntax(&mut self) -> Result<ZilSyntax, String> {
        let mut verb = String::new();
        let mut prep1 = None;
        let mut prep2 = None;
        let mut obj1_flags = Vec::new();
        let mut obj2_flags = Vec::new();
        let mut action = String::new();
        let mut preaction = None;
        let mut found_equals = false;
        let mut obj_count = 0;

        while !matches!(self.current_token, Token::RParen | Token::EOF) {
            match &self.current_token {
                Token::Symbol(s) if s == "=" => {
                    found_equals = true;
                    self.advance();
                }
                Token::Symbol(s) if !found_equals => {
                    if verb.is_empty() {
                        verb = s.clone();
                    } else if s == "OBJECT" {
                        obj_count += 1;
                    } else if obj_count == 0 {
                        // Could be a preposition
                        if prep1.is_none() {
                            prep1 = Some(s.clone());
                        } else if prep2.is_none() {
                            prep2 = Some(s.clone());
                        }
                    }
                    self.advance();
                }
                Token::Symbol(s) if found_equals => {
                    if action.is_empty() {
                        action = s.clone();
                    } else if preaction.is_none() {
                        preaction = Some(s.clone());
                    }
                    self.advance();
                }
                Token::LBracket => {
                    // Skip flag specifications
                    self.skip_to_close_bracket()?;
                }
                _ => self.advance(),
            }
        }

        if let Token::RParen = self.current_token {
            self.advance();
        }

        Ok(ZilSyntax {
            verb,
            prep1,
            prep2,
            obj1_flags,
            obj2_flags,
            action,
            preaction,
        })
    }

    fn parse_global(&mut self) -> Result<ZilGlobal, String> {
        let name = match &self.current_token {
            Token::Symbol(s) => s.clone(),
            _ => return Err(format!("Expected global name, got {:?}", self.current_token)),
        };
        self.advance();

        let value = if matches!(self.current_token, Token::RParen) {
            ZilValue::Nil
        } else {
            self.parse_value()?
        };

        // Skip to close
        while !matches!(self.current_token, Token::RParen | Token::EOF) {
            self.advance();
        }
        if let Token::RParen = self.current_token {
            self.advance();
        }

        Ok(ZilGlobal { name, value })
    }

    fn parse_constant(&mut self) -> Result<ZilConstant, String> {
        let name = match &self.current_token {
            Token::Symbol(s) => s.clone(),
            _ => return Err(format!("Expected constant name, got {:?}", self.current_token)),
        };
        self.advance();

        let value = if matches!(self.current_token, Token::RParen) {
            ZilValue::Nil
        } else {
            self.parse_value()?
        };

        while !matches!(self.current_token, Token::RParen | Token::EOF) {
            self.advance();
        }
        if let Token::RParen = self.current_token {
            self.advance();
        }

        Ok(ZilConstant { name, value })
    }

    fn parse_synonym(&mut self) -> Result<ZilDefinition, String> {
        let mut words = Vec::new();
        while !matches!(self.current_token, Token::RParen | Token::EOF) {
            if let Token::Symbol(s) = &self.current_token {
                words.push(s.clone());
            }
            self.advance();
        }
        if let Token::RParen = self.current_token {
            self.advance();
        }

        if words.is_empty() {
            return Err("Empty synonym definition".to_string());
        }

        let primary = words.remove(0);
        Ok(ZilDefinition::Synonym(primary, words))
    }

    fn parse_buzz(&mut self) -> Result<ZilDefinition, String> {
        let mut words = Vec::new();
        while !matches!(self.current_token, Token::RParen | Token::EOF) {
            if let Token::Symbol(s) = &self.current_token {
                words.push(s.clone());
            }
            self.advance();
        }
        if let Token::RParen = self.current_token {
            self.advance();
        }

        Ok(ZilDefinition::Buzz(words))
    }

    fn parse_directions(&mut self) -> Result<ZilDefinition, String> {
        let mut dirs = Vec::new();
        while !matches!(self.current_token, Token::RParen | Token::EOF) {
            if let Token::Symbol(s) = &self.current_token {
                dirs.push(s.clone());
            }
            self.advance();
        }
        if let Token::RParen = self.current_token {
            self.advance();
        }

        Ok(ZilDefinition::Directions(dirs))
    }

    fn parse_propdef(&mut self) -> Result<ZilDefinition, String> {
        let name = match &self.current_token {
            Token::Symbol(s) => s.clone(),
            _ => return Err(format!("Expected property name, got {:?}", self.current_token)),
        };
        self.advance();

        let value = if matches!(self.current_token, Token::RParen) {
            ZilValue::Number(0)
        } else {
            self.parse_value()?
        };

        while !matches!(self.current_token, Token::RParen | Token::EOF) {
            self.advance();
        }
        if let Token::RParen = self.current_token {
            self.advance();
        }

        Ok(ZilDefinition::PropertyDefault(name, value))
    }

    fn parse_version(&mut self) -> Result<ZilDefinition, String> {
        let version = match &self.current_token {
            Token::Symbol(s) => s.clone(),
            _ => "ZIP".to_string(),
        };
        self.skip_form()?;
        Ok(ZilDefinition::Version(version))
    }

    fn parse_insert_file(&mut self) -> Result<ZilDefinition, String> {
        let filename = match &self.current_token {
            Token::String(s) => s.clone(),
            Token::Symbol(s) => s.clone(),
            _ => return Err(format!("Expected filename, got {:?}", self.current_token)),
        };
        self.skip_form()?;
        Ok(ZilDefinition::InsertFile(filename))
    }

    fn parse_value(&mut self) -> Result<ZilValue, String> {
        match &self.current_token {
            Token::Number(n) => {
                let n = *n;
                self.advance();
                Ok(ZilValue::Number(n))
            }
            Token::String(s) => {
                let s = s.clone();
                self.advance();
                Ok(ZilValue::String(s))
            }
            Token::Symbol(s) => {
                let s = s.clone();
                self.advance();
                if s == "T" {
                    Ok(ZilValue::Number(1)) // T is true
                } else {
                    Ok(ZilValue::Symbol(s))
                }
            }
            Token::LocalVar(s) => {
                let s = s.clone();
                self.advance();
                Ok(ZilValue::LocalVar(s))
            }
            Token::GlobalVar(s) => {
                let s = s.clone();
                self.advance();
                Ok(ZilValue::GlobalVar(s))
            }
            Token::LParen => {
                self.advance();
                // Check if it's an empty form <>
                if let Token::RParen = self.current_token {
                    self.advance();
                    return Ok(ZilValue::Nil);
                }
                let mut items = Vec::new();
                while !matches!(self.current_token, Token::RParen | Token::EOF) {
                    items.push(self.parse_value()?);
                }
                if let Token::RParen = self.current_token {
                    self.advance();
                }
                Ok(ZilValue::Form(items))
            }
            Token::LBracket => {
                self.advance();
                let mut items = Vec::new();
                while !matches!(self.current_token, Token::RBracket | Token::EOF) {
                    items.push(self.parse_value()?);
                }
                if let Token::RBracket = self.current_token {
                    self.advance();
                }
                Ok(ZilValue::List(items))
            }
            Token::Quote => {
                self.advance();
                let val = self.parse_value()?;
                Ok(ZilValue::Quote(Box::new(val)))
            }
            Token::Comma => {
                self.advance();
                // Spliced global reference
                if let Token::Symbol(s) = &self.current_token {
                    let s = s.clone();
                    self.advance();
                    Ok(ZilValue::GlobalVar(s))
                } else {
                    Ok(ZilValue::Nil)
                }
            }
            Token::Dot => {
                self.advance();
                if let Token::Symbol(s) = &self.current_token {
                    let s = s.clone();
                    self.advance();
                    Ok(ZilValue::LocalVar(s))
                } else {
                    Ok(ZilValue::Nil)
                }
            }
            Token::Hash => {
                self.advance();
                // Handle #DECL, #BYTE, etc.
                if let Token::Symbol(s) = &self.current_token {
                    let _s = s.clone();
                    self.advance();
                    // Skip the following value
                    if !matches!(self.current_token, Token::RParen | Token::RBracket | Token::EOF) {
                        self.parse_value()?;
                    }
                }
                Ok(ZilValue::Nil)
            }
            Token::Percent => {
                self.advance();
                // Compile-time conditional
                self.parse_value()
            }
            _ => {
                self.advance();
                Ok(ZilValue::Nil)
            }
        }
    }
}
