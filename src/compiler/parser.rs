//! ZIL Parser
//!
//! Parses ZIL tokens into an AST.

use crate::compiler::lexer::{Token, TokenKind};
use crate::compiler::ast::*;

/// The ZIL parser
pub struct Parser {
    tokens: Vec<Token>,
    position: usize,
}

impl Parser {
    /// Create a new parser from tokens
    pub fn new(tokens: Vec<Token>) -> Self {
        Parser { tokens, position: 0 }
    }
    
    /// Parse all top-level forms
    pub fn parse(&mut self) -> Result<Vec<Form>, String> {
        let mut forms = Vec::new();
        
        while !self.is_at_end() {
            if let Some(form) = self.parse_top_level()? {
                forms.push(form);
            }
        }
        
        Ok(forms)
    }
    
    /// Parse a top-level form
    fn parse_top_level(&mut self) -> Result<Option<Form>, String> {
        // Skip standalone strings (comments/documentation)
        if matches!(self.peek_kind(), Some(TokenKind::String(_))) {
            self.advance();
            return Ok(None);
        }
        
        // Handle % reader macro - skip the conditional compilation
        if matches!(self.peek_kind(), Some(TokenKind::Percent)) {
            self.advance(); // consume %
            // The next form is a compile-time conditional - just parse and skip it
            if matches!(self.peek_kind(), Some(TokenKind::LAngle)) {
                self.skip_form()?;
            }
            return Ok(None);
        }
        
        if !matches!(self.peek_kind(), Some(TokenKind::LAngle)) {
            if self.is_at_end() {
                return Ok(None);
            }
            return Err(format!("Expected '<' at top level, got {:?}", self.peek()));
        }
        
        self.advance(); // consume <
        
        // Get the form name
        let form_name = match self.peek_kind() {
            Some(TokenKind::Atom(name)) => {
                let n = name.clone();
                self.advance();
                n
            }
            _ => return Err("Expected form name after '<'".to_string()),
        };
        
        let form = match form_name.as_str() {
            "CONSTANT" => self.parse_constant()?,
            "GLOBAL" => self.parse_global()?,
            "OBJECT" => self.parse_object()?,
            "ROOM" => self.parse_room()?,
            "ROUTINE" => self.parse_routine()?,
            "SYNTAX" => self.parse_syntax()?,
            "DEFMAC" | "DEFMACRO" => self.parse_defmacro()?,
            "DEFINE" => self.parse_define()?,
            "SETG" => self.parse_setg()?,
            "SET" => self.parse_set()?,
            "VERSION" => self.parse_version()?,
            "DIRECTIONS" => self.parse_directions()?,
            "INSERT-FILE" => self.parse_insert_file()?,
            "PROPDEF" => self.parse_propdef()?,
            "FREQUENT-WORDS?" => {
                self.expect_close()?;
                Form::FrequentWords
            }
            "BUZZ" => self.parse_buzz()?,
            "VERB-SYNONYM" => self.parse_verb_synonym()?,
            "OR" | "AND" | "COND" | "PROG" | "REPEAT" => {
                // These are control flow forms that appear at top level
                let args = self.parse_form_args()?;
                self.expect_close()?;
                Form::Other(form_name, args)
            }
            _ => {
                // Unknown form - parse generically
                let args = self.parse_form_args()?;
                self.expect_close()?;
                Form::Other(form_name, args)
            }
        };
        
        Ok(Some(form))
    }
    
    /// Parse <CONSTANT name value>
    fn parse_constant(&mut self) -> Result<Form, String> {
        let name = self.expect_atom()?;
        let value = self.parse_expr()?;
        self.expect_close()?;
        Ok(Form::Constant(name, value))
    }
    
    /// Parse <GLOBAL name value>
    fn parse_global(&mut self) -> Result<Form, String> {
        let name = self.expect_atom()?;
        let value = if matches!(self.peek_kind(), Some(TokenKind::RAngle)) {
            Expr::False
        } else {
            self.parse_expr()?
        };
        self.expect_close()?;
        Ok(Form::Global(name, value))
    }
    
    /// Parse <OBJECT name props...>
    fn parse_object(&mut self) -> Result<Form, String> {
        let name = self.expect_atom()?;
        let mut obj = ObjectDef::new(name);
        
        // Parse property lists
        while !matches!(self.peek_kind(), Some(TokenKind::RAngle)) {
            if matches!(self.peek_kind(), Some(TokenKind::LParen)) {
                self.advance(); // consume (
                let prop_name = self.expect_atom()?;
                let prop_value = self.parse_property_value(&prop_name)?;
                obj.properties.insert(prop_name, prop_value);
                self.expect_rparen()?;
            } else {
                break;
            }
        }
        
        self.expect_close()?;
        Ok(Form::Object(obj))
    }
    
    /// Parse <ROOM name props...>
    fn parse_room(&mut self) -> Result<Form, String> {
        let name = self.expect_atom()?;
        let mut room = RoomDef::new(name);
        
        // Parse property lists
        while !matches!(self.peek_kind(), Some(TokenKind::RAngle)) {
            if matches!(self.peek_kind(), Some(TokenKind::LParen)) {
                self.advance(); // consume (
                let prop_name = self.expect_atom()?;
                let prop_value = self.parse_property_value(&prop_name)?;
                room.properties.insert(prop_name, prop_value);
                self.expect_rparen()?;
            } else {
                break;
            }
        }
        
        self.expect_close()?;
        Ok(Form::Room(room))
    }
    
    /// Parse property value based on property name
    fn parse_property_value(&mut self, prop_name: &str) -> Result<Property, String> {
        match prop_name.to_uppercase().as_str() {
            "IN" | "LOC" => {
                // Check if this is a direction (IN TO room) or parent reference (IN object)
                if matches!(self.peek_kind(), Some(TokenKind::Atom(a)) if a == "TO" || a == "PER") {
                    // It's a direction
                    let dest = self.parse_exit_destination()?;
                    Ok(Property::Exit(ExitDef { 
                        direction: prop_name.to_string(),
                        destination: dest 
                    }))
                } else {
                    let parent = self.expect_atom()?;
                    Ok(Property::In(parent))
                }
            }
            "DESC" | "LDESC" | "FDESC" => {
                if matches!(self.peek_kind(), Some(TokenKind::String(_))) {
                    let s = self.expect_string()?;
                    Ok(Property::String(s))
                } else {
                    let expr = self.parse_expr()?;
                    Ok(Property::Expr(expr))
                }
            }
            "FLAGS" => {
                let mut flags = Vec::new();
                while !matches!(self.peek_kind(), Some(TokenKind::RParen)) {
                    flags.push(self.expect_atom()?);
                }
                Ok(Property::Flags(flags))
            }
            "SYNONYM" | "SYNONYMS" => {
                let mut syns = Vec::new();
                while !matches!(self.peek_kind(), Some(TokenKind::RParen)) {
                    syns.push(self.expect_atom()?);
                }
                Ok(Property::Synonym(syns))
            }
            "ADJECTIVE" | "ADJECTIVES" => {
                let mut adjs = Vec::new();
                while !matches!(self.peek_kind(), Some(TokenKind::RParen)) {
                    if matches!(self.peek_kind(), Some(TokenKind::String(_))) {
                        // Skip string comments in adjective list
                        self.advance();
                    } else if let Some(TokenKind::Atom(_)) = self.peek_kind() {
                        adjs.push(self.expect_atom()?);
                    } else {
                        // Unknown token type - break out
                        break;
                    }
                }
                Ok(Property::Adjective(adjs))
            }
            "ACTION" => {
                // Action can be a routine name or 0
                if matches!(self.peek_kind(), Some(TokenKind::Number(0))) {
                    self.advance();
                    Ok(Property::Number(0))
                } else {
                    let action = self.expect_atom()?;
                    Ok(Property::Action(action))
                }
            }
            "DESCFCN" | "CONTFCN" | "ADVFCN" => {
                // Function can be a routine name or 0
                if matches!(self.peek_kind(), Some(TokenKind::Number(0))) {
                    self.advance();
                    Ok(Property::Number(0))
                } else {
                    let fcn = self.expect_atom()?;
                    Ok(Property::DescFcn(fcn))
                }
            }
            "GLOBAL" => {
                let mut items = Vec::new();
                while !matches!(self.peek_kind(), Some(TokenKind::RParen)) {
                    items.push(self.parse_expr()?);
                }
                Ok(Property::List(items))
            }
            "PSEUDO" => {
                let mut pairs = Vec::new();
                while !matches!(self.peek_kind(), Some(TokenKind::RParen)) {
                    let word = if matches!(self.peek_kind(), Some(TokenKind::String(_))) {
                        self.expect_string()?
                    } else {
                        self.expect_atom()?
                    };
                    let action = self.expect_atom()?;
                    pairs.push((word, action));
                }
                Ok(Property::Pseudo(pairs))
            }
            "SIZE" | "VALUE" | "TVALUE" | "CAPACITY" | "STRENGTH" | "VTYPE" => {
                if matches!(self.peek_kind(), Some(TokenKind::Number(_))) {
                    let n = self.expect_number()?;
                    Ok(Property::Number(n))
                } else {
                    let expr = self.parse_expr()?;
                    Ok(Property::Expr(expr))
                }
            }
            // Direction properties for rooms
            "NORTH" | "SOUTH" | "EAST" | "WEST" | "NE" | "NW" | "SE" | "SW" 
            | "UP" | "DOWN" | "IN" | "OUT" | "LAND" => {
                let dest = self.parse_exit_destination()?;
                Ok(Property::Exit(ExitDef { 
                    direction: prop_name.to_string(),
                    destination: dest 
                }))
            }
            _ => {
                // Generic property - parse as expression list
                let mut items = Vec::new();
                while !matches!(self.peek_kind(), Some(TokenKind::RParen)) {
                    items.push(self.parse_expr()?);
                }
                if items.len() == 1 {
                    Ok(Property::Expr(items.remove(0)))
                } else {
                    Ok(Property::List(items))
                }
            }
        }
    }
    
    /// Parse exit destination
    fn parse_exit_destination(&mut self) -> Result<ExitDestination, String> {
        // ZIL exit formats:
        // - TO ROOM
        // - TO ROOM IF FLAG
        // - TO ROOM IF FLAG ELSE STRING
        // - PER ROUTINE
        // - SORRY "message"
        // - "message"
        // - #NEXIT
        
        // Check for string message first
        if let Some(TokenKind::String(msg)) = self.peek_kind() {
            let message = msg.clone();
            self.advance();
            return Ok(ExitDestination::Message(message));
        }
        
        // Skip all tokens until the closing paren
        // For now, we just consume everything
        while !matches!(self.peek_kind(), Some(TokenKind::RParen)) {
            self.advance();
        }
        
        Ok(ExitDestination::Room("PLACEHOLDER".to_string()))
    }
    
    /// Parse <ROUTINE name (args) body...>
    fn parse_routine(&mut self) -> Result<Form, String> {
        let name = self.expect_atom()?;
        let args = self.parse_arg_list()?;
        
        // Parse body
        let mut body = Vec::new();
        while !matches!(self.peek_kind(), Some(TokenKind::RAngle)) {
            body.push(self.parse_expr()?);
        }
        
        self.expect_close()?;
        
        Ok(Form::Routine(RoutineDef { name, args, body }))
    }
    
    /// Parse argument list (args "OPTIONAL" opt-args "AUX" aux-vars)
    fn parse_arg_list(&mut self) -> Result<Vec<ArgDef>, String> {
        self.expect_lparen()?;
        
        let mut args = Vec::new();
        let mut arg_type = ArgType::Required;
        
        while !matches!(self.peek_kind(), Some(TokenKind::RParen)) {
            match self.peek_kind() {
                Some(TokenKind::String(s)) => {
                    let s = s.clone();
                    self.advance();
                    match s.to_uppercase().as_str() {
                        "OPTIONAL" => arg_type = ArgType::Optional,
                        "OPT" => arg_type = ArgType::Optional,
                        "AUX" => arg_type = ArgType::Aux,
                        "ARGS" => arg_type = ArgType::Args,
                        _ => {}
                    }
                }
                Some(TokenKind::Atom(_)) => {
                    let name = self.expect_atom()?;
                    
                    // Check for default value in parentheses
                    let default = if matches!(self.peek_kind(), Some(TokenKind::LParen)) {
                        self.advance();
                        let name_in_paren = self.expect_atom()?;
                        let default_val = self.parse_expr()?;
                        self.expect_rparen()?;
                        Some(default_val)
                    } else {
                        None
                    };
                    
                    args.push(ArgDef {
                        name,
                        arg_type: arg_type.clone(),
                        default,
                    });
                }
                Some(TokenKind::LParen) => {
                    // Argument with default value
                    self.advance();
                    let name = self.expect_atom()?;
                    let default = if !matches!(self.peek_kind(), Some(TokenKind::RParen)) {
                        Some(self.parse_expr()?)
                    } else {
                        None
                    };
                    self.expect_rparen()?;
                    
                    args.push(ArgDef {
                        name,
                        arg_type: arg_type.clone(),
                        default,
                    });
                }
                _ => break,
            }
        }
        
        self.expect_rparen()?;
        Ok(args)
    }
    
    /// Parse <SYNTAX ...>
    fn parse_syntax(&mut self) -> Result<Form, String> {
        // For now, skip syntax definitions
        while !matches!(self.peek_kind(), Some(TokenKind::RAngle)) {
            self.advance();
        }
        self.expect_close()?;
        Ok(Form::Syntax(SyntaxDef {
            verb: String::new(),
            patterns: Vec::new(),
        }))
    }
    
    /// Parse <DEFMAC name (args) body>
    fn parse_defmacro(&mut self) -> Result<Form, String> {
        let name = self.expect_atom()?;
        
        // Parse args
        let mut args = Vec::new();
        if matches!(self.peek_kind(), Some(TokenKind::LParen)) {
            self.advance();
            while !matches!(self.peek_kind(), Some(TokenKind::RParen)) {
                if let Some(TokenKind::Atom(a)) = self.peek_kind() {
                    args.push(a.clone());
                    self.advance();
                } else if let Some(TokenKind::String(s)) = self.peek_kind() {
                    args.push(s.clone());
                    self.advance();
                } else if matches!(self.peek_kind(), Some(TokenKind::Quote)) {
                    self.advance();
                    if let Some(TokenKind::Atom(a)) = self.peek_kind() {
                        args.push(format!("'{}", a));
                        self.advance();
                    }
                } else {
                    self.advance();
                }
            }
            self.expect_rparen()?;
        }
        
        // Parse body
        let mut body = Vec::new();
        while !matches!(self.peek_kind(), Some(TokenKind::RAngle)) {
            body.push(self.parse_expr()?);
        }
        
        self.expect_close()?;
        Ok(Form::DefMacro(name, args, body))
    }
    
    /// Parse <DEFINE name ...>
    fn parse_define(&mut self) -> Result<Form, String> {
        let name = self.expect_atom()?;
        
        // Parse args
        let args = if matches!(self.peek_kind(), Some(TokenKind::LParen)) {
            self.parse_arg_list()?
        } else {
            Vec::new()
        };
        
        // Parse body
        let mut body = Vec::new();
        while !matches!(self.peek_kind(), Some(TokenKind::RAngle)) {
            body.push(self.parse_expr()?);
        }
        
        self.expect_close()?;
        Ok(Form::Routine(RoutineDef { name, args, body }))
    }
    
    /// Parse <SETG name value>
    fn parse_setg(&mut self) -> Result<Form, String> {
        let name = self.expect_atom()?;
        let value = self.parse_expr()?;
        self.expect_close()?;
        Ok(Form::SetG(name, value))
    }
    
    /// Parse <SET name value>
    fn parse_set(&mut self) -> Result<Form, String> {
        let name = self.expect_atom()?;
        let value = self.parse_expr()?;
        self.expect_close()?;
        // SET is similar to SETG for our purposes
        Ok(Form::SetG(name, value))
    }
    
    /// Parse <VERSION zip>
    fn parse_version(&mut self) -> Result<Form, String> {
        let ver = self.expect_atom()?;
        self.expect_close()?;
        Ok(Form::Version(ver))
    }
    
    /// Parse <DIRECTIONS dir...>
    fn parse_directions(&mut self) -> Result<Form, String> {
        let mut dirs = Vec::new();
        while !matches!(self.peek_kind(), Some(TokenKind::RAngle)) {
            dirs.push(self.expect_atom()?);
        }
        self.expect_close()?;
        Ok(Form::Directions(dirs))
    }
    
    /// Parse <INSERT-FILE filename flag?>
    fn parse_insert_file(&mut self) -> Result<Form, String> {
        let filename = if matches!(self.peek_kind(), Some(TokenKind::String(_))) {
            self.expect_string()?
        } else {
            self.expect_atom()?
        };
        
        let flag = if !matches!(self.peek_kind(), Some(TokenKind::RAngle)) {
            self.advance();
            true
        } else {
            false
        };
        
        self.expect_close()?;
        Ok(Form::InsertFile(filename, flag))
    }
    
    /// Parse <PROPDEF name default>
    fn parse_propdef(&mut self) -> Result<Form, String> {
        let name = self.expect_atom()?;
        let default = self.expect_number()?;
        self.expect_close()?;
        Ok(Form::PropDef(name, default))
    }
    
    /// Parse <BUZZ word...>
    fn parse_buzz(&mut self) -> Result<Form, String> {
        let mut words = Vec::new();
        while !matches!(self.peek_kind(), Some(TokenKind::RAngle)) {
            words.push(self.expect_atom()?);
        }
        self.expect_close()?;
        Ok(Form::Buzz(words))
    }
    
    /// Parse <VERB-SYNONYM verb syn...>
    fn parse_verb_synonym(&mut self) -> Result<Form, String> {
        let verb = self.expect_atom()?;
        let mut syns = Vec::new();
        while !matches!(self.peek_kind(), Some(TokenKind::RAngle)) {
            syns.push(self.expect_atom()?);
        }
        self.expect_close()?;
        Ok(Form::VerbSynonym(verb, syns))
    }
    
    /// Parse form arguments until closing >
    fn parse_form_args(&mut self) -> Result<Vec<Expr>, String> {
        let mut args = Vec::new();
        while !matches!(self.peek_kind(), Some(TokenKind::RAngle)) {
            args.push(self.parse_expr()?);
        }
        Ok(args)
    }
    
    /// Parse an expression
    fn parse_expr(&mut self) -> Result<Expr, String> {
        match self.peek_kind() {
            Some(TokenKind::Percent) => {
                // Reader macro - skip the conditional
                self.advance();
                self.parse_expr()
            }
            Some(TokenKind::LAngle) => self.parse_form_expr(),
            Some(TokenKind::LParen) => self.parse_list(),
            Some(TokenKind::LBracket) => self.parse_table(),
            Some(TokenKind::Quote) => {
                self.advance();
                let expr = self.parse_expr()?;
                Ok(Expr::Quote(Box::new(expr)))
            }
            Some(TokenKind::FormQuote) => {
                self.advance();
                // Parse the form after '<
                let expr = self.parse_form_expr()?;
                Ok(Expr::Quote(Box::new(expr)))
            }
            Some(TokenKind::Atom(name)) => {
                let n = name.clone();
                self.advance();
                if n == "T" {
                    Ok(Expr::True)
                } else {
                    Ok(Expr::Atom(n))
                }
            }
            Some(TokenKind::Number(n)) => {
                let num = *n;
                self.advance();
                Ok(Expr::Number(num))
            }
            Some(TokenKind::String(s)) => {
                let str = s.clone();
                self.advance();
                Ok(Expr::String(str))
            }
            Some(TokenKind::GVal(name)) => {
                let n = name.clone();
                self.advance();
                Ok(Expr::GVal(n))
            }
            Some(TokenKind::LVal(name)) => {
                let n = name.clone();
                self.advance();
                Ok(Expr::LVal(n))
            }
            Some(TokenKind::Bang) => {
                // Splice operator
                self.advance();
                let expr = self.parse_expr()?;
                Ok(Expr::Form("SPLICE".to_string(), vec![expr]))
            }
            Some(TokenKind::Comma) => {
                // Lone comma - global reference follows
                self.advance();
                let name = self.expect_atom()?;
                Ok(Expr::GVal(name))
            }
            Some(TokenKind::Period) => {
                // Lone period - local reference follows
                self.advance();
                let name = self.expect_atom()?;
                Ok(Expr::LVal(name))
            }
            Some(TokenKind::RAngle) | Some(TokenKind::RParen) | Some(TokenKind::RBracket) => {
                // Empty expression (false)
                Ok(Expr::False)
            }
            _ => Err(format!("Unexpected token in expression: {:?}", self.peek())),
        }
    }
    
    /// Parse a form expression <...>
    fn parse_form_expr(&mut self) -> Result<Expr, String> {
        self.advance(); // consume <
        
        // Check for empty form <>
        if matches!(self.peek_kind(), Some(TokenKind::RAngle)) {
            self.advance();
            return Ok(Expr::False);
        }
        
        // Get form name
        let name = match self.peek_kind() {
            Some(TokenKind::Atom(n)) => {
                let name = n.clone();
                self.advance();
                name
            }
            Some(TokenKind::GVal(n)) => {
                // ,PRSA etc. can be form names in macros
                let name = n.clone();
                self.advance();
                format!(",{}", name)
            }
            _ => return Err("Expected form name".to_string()),
        };
        
        // Parse arguments
        let mut args = Vec::new();
        while !matches!(self.peek_kind(), Some(TokenKind::RAngle)) {
            args.push(self.parse_expr()?);
        }
        
        self.expect_close()?;
        Ok(Expr::Form(name, args))
    }
    
    /// Parse a list (...)
    fn parse_list(&mut self) -> Result<Expr, String> {
        self.advance(); // consume (
        
        let mut items = Vec::new();
        while !matches!(self.peek_kind(), Some(TokenKind::RParen)) {
            items.push(self.parse_expr()?);
        }
        
        self.expect_rparen()?;
        Ok(Expr::List(items))
    }
    
    /// Parse a table [...]
    fn parse_table(&mut self) -> Result<Expr, String> {
        self.advance(); // consume [
        
        let mut items = Vec::new();
        while !matches!(self.peek_kind(), Some(TokenKind::RBracket)) {
            items.push(self.parse_expr()?);
        }
        
        self.expect_rbracket()?;
        Ok(Expr::Table(items))
    }
    
    // Helper methods
    
    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.position)
    }
    
    fn peek_kind(&self) -> Option<&TokenKind> {
        self.peek().map(|t| &t.kind)
    }
    
    fn advance(&mut self) -> Option<&Token> {
        if self.position < self.tokens.len() {
            self.position += 1;
            self.tokens.get(self.position - 1)
        } else {
            None
        }
    }
    
    fn is_at_end(&self) -> bool {
        matches!(self.peek_kind(), Some(TokenKind::Eof) | None)
    }
    
    fn expect_atom(&mut self) -> Result<String, String> {
        match self.peek_kind() {
            Some(TokenKind::Atom(name)) => {
                let n = name.clone();
                self.advance();
                Ok(n)
            }
            _ => Err(format!("Expected atom, got {:?}", self.peek())),
        }
    }
    
    fn expect_number(&mut self) -> Result<i32, String> {
        match self.peek_kind() {
            Some(TokenKind::Number(n)) => {
                let num = *n;
                self.advance();
                Ok(num)
            }
            _ => Err(format!("Expected number, got {:?}", self.peek())),
        }
    }
    
    fn expect_string(&mut self) -> Result<String, String> {
        match self.peek_kind() {
            Some(TokenKind::String(s)) => {
                let str = s.clone();
                self.advance();
                Ok(str)
            }
            _ => Err(format!("Expected string, got {:?}", self.peek())),
        }
    }
    
    fn expect_close(&mut self) -> Result<(), String> {
        match self.peek_kind() {
            Some(TokenKind::RAngle) => {
                self.advance();
                Ok(())
            }
            _ => Err(format!("Expected '>', got {:?}", self.peek())),
        }
    }
    
    fn expect_lparen(&mut self) -> Result<(), String> {
        match self.peek_kind() {
            Some(TokenKind::LParen) => {
                self.advance();
                Ok(())
            }
            _ => Err(format!("Expected '(', got {:?}", self.peek())),
        }
    }
    
    fn expect_rparen(&mut self) -> Result<(), String> {
        match self.peek_kind() {
            Some(TokenKind::RParen) => {
                self.advance();
                Ok(())
            }
            _ => Err(format!("Expected ')', got {:?}", self.peek())),
        }
    }
    
    fn expect_rbracket(&mut self) -> Result<(), String> {
        match self.peek_kind() {
            Some(TokenKind::RBracket) => {
                self.advance();
                Ok(())
            }
            _ => Err(format!("Expected ']', got {:?}", self.peek())),
        }
    }
    
    /// Skip a form completely (for compile-time conditionals)
    fn skip_form(&mut self) -> Result<(), String> {
        if !matches!(self.peek_kind(), Some(TokenKind::LAngle)) {
            return Ok(());
        }
        
        self.advance(); // consume <
        let mut depth = 1;
        
        while depth > 0 && !self.is_at_end() {
            match self.peek_kind() {
                Some(TokenKind::LAngle) => {
                    depth += 1;
                }
                Some(TokenKind::RAngle) => {
                    depth -= 1;
                }
                _ => {}
            }
            self.advance();
        }
        
        Ok(())
    }
}
