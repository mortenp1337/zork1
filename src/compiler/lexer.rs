//! ZIL Lexer - Tokenizer for ZIL source code
//!
//! ZIL is a LISP-like language, so tokens include:
//! - Atoms (identifiers)
//! - Numbers
//! - Strings
//! - Special characters: < > ( ) [ ] ' , . ; !

use std::iter::Peekable;
use std::str::Chars;

/// Token types
#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    /// Opening angle bracket <
    LAngle,
    /// Closing angle bracket >
    RAngle,
    /// Opening parenthesis (
    LParen,
    /// Closing parenthesis )
    RParen,
    /// Opening square bracket [
    LBracket,
    /// Closing square bracket ]
    RBracket,
    /// Quote '
    Quote,
    /// Comma ,
    Comma,
    /// Period .
    Period,
    /// Exclamation mark (for splicing) !
    Bang,
    /// Atom (identifier)
    Atom(String),
    /// Integer number
    Number(i32),
    /// String literal
    String(String),
    /// Global value reference ,var
    GVal(String),
    /// Local value reference .var
    LVal(String),
    /// Form quotation '<...>
    FormQuote,
    /// Reader macro prefix %
    Percent,
    /// End of file
    Eof,
}

/// A token with position information
#[derive(Debug, Clone)]
pub struct Token {
    pub kind: TokenKind,
    pub line: usize,
    pub column: usize,
}

impl Token {
    pub fn new(kind: TokenKind, line: usize, column: usize) -> Self {
        Token { kind, line, column }
    }
}

/// The ZIL lexer
pub struct Lexer<'a> {
    source: &'a str,
    chars: Peekable<Chars<'a>>,
    line: usize,
    column: usize,
    position: usize,
}

impl<'a> Lexer<'a> {
    /// Create a new lexer for the given source
    pub fn new(source: &'a str) -> Self {
        Lexer {
            source,
            chars: source.chars().peekable(),
            line: 1,
            column: 1,
            position: 0,
        }
    }
    
    /// Tokenize the entire source
    pub fn tokenize(&mut self) -> Result<Vec<Token>, String> {
        let mut tokens = Vec::new();
        
        loop {
            let token = self.next_token()?;
            let is_eof = token.kind == TokenKind::Eof;
            tokens.push(token);
            if is_eof {
                break;
            }
        }
        
        Ok(tokens)
    }
    
    /// Get the next token
    fn next_token(&mut self) -> Result<Token, String> {
        self.skip_whitespace_and_comments();
        
        let line = self.line;
        let column = self.column;
        
        match self.peek() {
            None => Ok(Token::new(TokenKind::Eof, line, column)),
            Some(&c) => {
                match c {
                    '<' => {
                        self.advance();
                        Ok(Token::new(TokenKind::LAngle, line, column))
                    }
                    '>' => {
                        self.advance();
                        Ok(Token::new(TokenKind::RAngle, line, column))
                    }
                    '(' => {
                        self.advance();
                        Ok(Token::new(TokenKind::LParen, line, column))
                    }
                    ')' => {
                        self.advance();
                        Ok(Token::new(TokenKind::RParen, line, column))
                    }
                    '[' => {
                        self.advance();
                        Ok(Token::new(TokenKind::LBracket, line, column))
                    }
                    ']' => {
                        self.advance();
                        Ok(Token::new(TokenKind::RBracket, line, column))
                    }
                    '\'' => {
                        self.advance();
                        // Check if this is a form quote '<
                        if self.peek() == Some(&'<') {
                            Ok(Token::new(TokenKind::FormQuote, line, column))
                        } else {
                            Ok(Token::new(TokenKind::Quote, line, column))
                        }
                    }
                    ',' => {
                        self.advance();
                        // Check for global value reference
                        if let Some(&next_c) = self.peek() {
                            if Self::is_atom_start(next_c) {
                                let name = self.read_atom();
                                return Ok(Token::new(TokenKind::GVal(name), line, column));
                            }
                        }
                        Ok(Token::new(TokenKind::Comma, line, column))
                    }
                    '.' => {
                        self.advance();
                        // Check for local value reference
                        if let Some(&next_c) = self.peek() {
                            if Self::is_atom_start(next_c) {
                                let name = self.read_atom();
                                return Ok(Token::new(TokenKind::LVal(name), line, column));
                            }
                        }
                        Ok(Token::new(TokenKind::Period, line, column))
                    }
                    '!' => {
                        self.advance();
                        // Check for !\", which is a quoted quote character
                        if self.peek() == Some(&'\\') {
                            self.advance();
                            if self.peek() == Some(&'"') {
                                self.advance();
                                return Ok(Token::new(TokenKind::Atom("QUOTE-CHAR".to_string()), line, column));
                            }
                        }
                        Ok(Token::new(TokenKind::Bang, line, column))
                    }
                    '"' => {
                        let s = self.read_string()?;
                        Ok(Token::new(TokenKind::String(s), line, column))
                    }
                    '-' | '0'..='9' => {
                        // Could be number or atom starting with -
                        let atom = self.read_atom();
                        if let Ok(n) = atom.parse::<i32>() {
                            Ok(Token::new(TokenKind::Number(n), line, column))
                        } else {
                            Ok(Token::new(TokenKind::Atom(atom), line, column))
                        }
                    }
                    '%' => {
                        // Reader macro prefix
                        self.advance();
                        Ok(Token::new(TokenKind::Percent, line, column))
                    }
                    _ if Self::is_atom_start(c) => {
                        let atom = self.read_atom();
                        // Check for special atoms that are numbers in different bases
                        if atom.starts_with("#") {
                            // Hex, octal, etc.
                            Ok(Token::new(TokenKind::Atom(atom), line, column))
                        } else {
                            Ok(Token::new(TokenKind::Atom(atom), line, column))
                        }
                    }
                    _ => {
                        Err(format!("Unexpected character '{}' at line {}, column {}", c, line, column))
                    }
                }
            }
        }
    }
    
    /// Skip whitespace and comments
    fn skip_whitespace_and_comments(&mut self) {
        loop {
            // Skip whitespace
            while let Some(&c) = self.peek() {
                if c.is_whitespace() {
                    self.advance();
                } else {
                    break;
                }
            }
            
            // Skip standalone backslash at start of line (artifact)
            if self.peek() == Some(&'\\') && self.column == 1 {
                self.advance();
                // Skip to end of line
                while let Some(&c) = self.peek() {
                    if c == '\n' {
                        break;
                    }
                    self.advance();
                }
                continue;
            }
            
            // Skip /^L form feed markers at start of line
            if self.peek() == Some(&'/') && self.column == 1 {
                self.advance();
                // Check for ^L or just skip the line
                while let Some(&c) = self.peek() {
                    if c == '\n' {
                        break;
                    }
                    self.advance();
                }
                continue;
            }
            
            // Skip comments
            if self.peek() == Some(&';') {
                self.advance();
                // Check if it's a quoted comment ;"...."
                if self.peek() == Some(&'"') {
                    // Skip the quoted string comment
                    self.advance(); // skip "
                    while let Some(&c) = self.peek() {
                        self.advance();
                        if c == '"' {
                            break;
                        }
                    }
                } else if self.peek() == Some(&'<') {
                    // Form comment ;<...> - skip the entire form
                    self.advance(); // skip <
                    let mut depth = 1;
                    while depth > 0 {
                        match self.advance() {
                            Some('<') => depth += 1,
                            Some('>') => depth -= 1,
                            None => break,
                            _ => {}
                        }
                    }
                } else if self.peek() == Some(&'(') {
                    // List comment ;(...) - skip the entire list
                    self.advance(); // skip (
                    let mut depth = 1;
                    while depth > 0 {
                        match self.advance() {
                            Some('(') => depth += 1,
                            Some(')') => depth -= 1,
                            None => break,
                            _ => {}
                        }
                    }
                } else if self.peek() == Some(&',') || self.peek() == Some(&'.') {
                    // Comment out a single atom (;,GLOBAL or ;.LOCAL)
                    self.advance(); // skip , or .
                    // Skip the atom name
                    while let Some(&c) = self.peek() {
                        if c.is_whitespace() || matches!(c, '>' | ')' | ']' | '<' | '(' | '[') {
                            break;
                        }
                        self.advance();
                    }
                } else if let Some(&c) = self.peek() {
                    if c.is_alphabetic() || c == '-' || c == '_' || c == '?' {
                        // Comment out a single atom
                        while let Some(&c) = self.peek() {
                            if c.is_whitespace() || matches!(c, '>' | ')' | ']' | '<' | '(' | '[') {
                                break;
                            }
                            self.advance();
                        }
                    } else {
                        // Single-line comment - skip to end of line
                        while let Some(&c) = self.peek() {
                            if c == '\n' {
                                break;
                            }
                            self.advance();
                        }
                    }
                }
            } else if self.peek() == Some(&'^') {
                // Page break marker (^L)
                self.advance();
                if self.peek() == Some(&'L') {
                    self.advance();
                    // Skip to end of line
                    while let Some(&c) = self.peek() {
                        if c == '\n' {
                            break;
                        }
                        self.advance();
                    }
                } else {
                    // Not a page break, ^ might be part of an atom
                    // We'll handle it in the main tokenizer
                    break;
                }
            } else if self.peek() == Some(&'/') {
                // Possible page break (form feed indicator)
                // Peek ahead without consuming
                // Since we can't peek two chars, we need a different approach
                // Just check and handle in the main tokenizer
                break;
            } else {
                break;
            }
        }
    }
    
    /// Peek at the next character
    fn peek(&mut self) -> Option<&char> {
        self.chars.peek()
    }
    
    /// Advance to the next character
    fn advance(&mut self) -> Option<char> {
        let c = self.chars.next()?;
        self.position += c.len_utf8();
        if c == '\n' {
            self.line += 1;
            self.column = 1;
        } else {
            self.column += 1;
        }
        Some(c)
    }
    
    /// Check if a character can start an atom
    fn is_atom_start(c: char) -> bool {
        c.is_alphabetic() || matches!(c, '_' | '-' | '+' | '*' | '/' | '=' | '?' | '!' | '@' | '#' | '$' | '^' | '&' | '~' | '\\' | ':')
    }
    
    /// Check if a character can be part of an atom
    fn is_atom_char(c: char) -> bool {
        Self::is_atom_start(c) || c.is_numeric() || c == '-' || c == ':'
    }
    
    /// Read an atom (identifier)
    fn read_atom(&mut self) -> String {
        let mut atom = String::new();
        
        while let Some(&c) = self.peek() {
            if Self::is_atom_char(c) || c.is_numeric() {
                atom.push(c);
                self.advance();
                // Handle backslash escapes in atoms (like \. \, \")
                if c == '\\' {
                    if let Some(&next) = self.peek() {
                        if matches!(next, '.' | ',' | '"' | '\'' | '<' | '>' | '#') {
                            atom.push(next);
                            self.advance();
                        }
                    }
                }
            } else {
                break;
            }
        }
        
        atom.to_uppercase()
    }
    
    /// Read a string literal
    fn read_string(&mut self) -> Result<String, String> {
        let start_line = self.line;
        // Skip opening quote
        self.advance();
        
        let mut s = String::new();
        
        loop {
            match self.advance() {
                None => return Err(format!("Unterminated string starting at line {}", start_line)),
                Some('"') => break,
                Some('\\') => {
                    // Escape sequence
                    match self.advance() {
                        Some('n') => s.push('\n'),
                        Some('t') => s.push('\t'),
                        Some('r') => s.push('\r'),
                        Some('"') => s.push('"'),
                        Some('\\') => s.push('\\'),
                        Some(c) => s.push(c),
                        None => return Err("Unterminated escape sequence".to_string()),
                    }
                }
                Some(c) => s.push(c),
            }
        }
        
        Ok(s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_basic_tokens() {
        let mut lexer = Lexer::new("<CONSTANT FOO 42>");
        let tokens = lexer.tokenize().unwrap();
        
        assert_eq!(tokens.len(), 5); // < CONSTANT FOO 42 > EOF
        assert!(matches!(tokens[0].kind, TokenKind::LAngle));
        assert!(matches!(&tokens[1].kind, TokenKind::Atom(s) if s == "CONSTANT"));
        assert!(matches!(&tokens[2].kind, TokenKind::Atom(s) if s == "FOO"));
        assert!(matches!(tokens[3].kind, TokenKind::Number(42)));
        assert!(matches!(tokens[4].kind, TokenKind::RAngle));
    }
    
    #[test]
    fn test_string_literal() {
        let mut lexer = Lexer::new("\"Hello, World!\"");
        let tokens = lexer.tokenize().unwrap();
        
        assert_eq!(tokens.len(), 2); // string EOF
        assert!(matches!(&tokens[0].kind, TokenKind::String(s) if s == "Hello, World!"));
    }
}
