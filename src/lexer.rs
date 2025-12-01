/// ZIL Lexer - Tokenizes ZIL source code
use std::iter::Peekable;
use std::str::Chars;

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    // Delimiters
    LParen,    // <
    RParen,    // >
    LBracket,  // (
    RBracket,  // )
    LBrace,    // {
    RBrace,    // }

    // Literals
    String(String),
    Number(i64),
    Symbol(String),
    Atom(String),

    // Special characters
    Comma,     // ,
    Quote,     // '
    Dot,       // .
    Hash,      // #
    Percent,   // %
    Semicolon, // ;
    Backslash, // \
    Pipe,      // |
    Caret,     // ^

    // Special forms
    LocalVar(String),  // .VAR
    GlobalVar(String), // ,VAR

    // EOF
    EOF,
}

pub struct Lexer<'a> {
    input: Peekable<Chars<'a>>,
    pub line: usize,
    pub column: usize,
    token_count: usize,
}

impl<'a> Lexer<'a> {
    pub fn new(input: &'a str) -> Self {
        Lexer {
            input: input.chars().peekable(),
            line: 1,
            column: 1,
            token_count: 0,
        }
    }

    fn advance(&mut self) -> Option<char> {
        let ch = self.input.next();
        if let Some(c) = ch {
            if c == '\n' {
                self.line += 1;
                self.column = 1;
            } else {
                self.column += 1;
            }
        }
        ch
    }

    fn peek(&mut self) -> Option<&char> {
        self.input.peek()
    }

    fn skip_whitespace(&mut self) {
        while let Some(&ch) = self.peek() {
            if ch.is_whitespace() {
                self.advance();
            } else {
                break;
            }
        }
    }

    fn skip_comment(&mut self) {
        // ZIL comments start with ; and go to end of line
        self.advance(); // consume the semicolon
        while let Some(&ch) = self.peek() {
            if ch == '\n' {
                break;
            }
            self.advance();
        }
    }

    fn read_string(&mut self) -> Token {
        let mut s = String::new();
        self.advance(); // consume opening quote
        let mut char_count = 0;
        const MAX_STRING_LEN: usize = 100000;
        
        while let Some(&ch) = self.peek() {
            char_count += 1;
            if char_count > MAX_STRING_LEN {
                // String too long, probably unclosed
                break;
            }
            
            if ch == '"' {
                self.advance(); // consume closing quote
                break;
            } else if ch == '|' {
                // | in ZIL strings represents newline
                self.advance();
                s.push('\n');
            } else if ch == '\\' {
                self.advance();
                if let Some(&escaped) = self.peek() {
                    match escaped {
                        'n' => s.push('\n'),
                        't' => s.push('\t'),
                        '"' => s.push('"'),
                        '\\' => s.push('\\'),
                        _ => s.push(escaped),
                    }
                    self.advance();
                }
            } else {
                s.push(ch);
                self.advance();
            }
        }
        Token::String(s)
    }

    fn read_number(&mut self) -> Token {
        let mut num = String::new();
        let mut negative = false;

        if let Some(&'-') = self.peek() {
            negative = true;
            self.advance();
        }

        while let Some(&ch) = self.peek() {
            if ch.is_ascii_digit() {
                num.push(ch);
                self.advance();
            } else {
                break;
            }
        }

        let value: i64 = num.parse().unwrap_or(0);
        Token::Number(if negative { -value } else { value })
    }

    fn read_symbol(&mut self) -> String {
        let mut sym = String::new();
        let mut char_count = 0;
        const MAX_SYMBOL_LEN: usize = 1000;
        
        while let Some(&ch) = self.peek() {
            char_count += 1;
            if char_count > MAX_SYMBOL_LEN {
                break;
            }
            
            if ch.is_alphanumeric()
                || ch == '-'
                || ch == '_'
                || ch == '?'
                || ch == '!'
                || ch == '#'
                || ch == '/'
                || ch == '*'
                || ch == '+'
                || ch == '='
                || ch == '\\'
                || ch == '$'
            {
                sym.push(ch);
                self.advance();
            } else {
                break;
            }
        }
        sym.to_uppercase()
    }

    fn read_quoted_symbol(&mut self) -> String {
        let mut sym = String::new();
        self.advance(); // consume backslash
        while let Some(&ch) = self.peek() {
            if ch.is_whitespace() || ch == '<' || ch == '>' || ch == '(' || ch == ')' {
                break;
            }
            sym.push(ch);
            self.advance();
        }
        sym
    }

    pub fn next_token(&mut self) -> Token {
        self.token_count += 1;
        if self.token_count > 2000000 {
            // Safety limit
            eprintln!("Lexer: hit token limit at line {}", self.line);
            return Token::EOF;
        }
        
        self.skip_whitespace();

        // Check for comments
        while let Some(&';') = self.peek() {
            self.skip_comment();
            self.skip_whitespace();
        }

        match self.peek() {
            None => Token::EOF,
            Some(&ch) => match ch {
                '<' => {
                    self.advance();
                    Token::LParen
                }
                '>' => {
                    self.advance();
                    Token::RParen
                }
                '(' => {
                    self.advance();
                    Token::LBracket
                }
                ')' => {
                    self.advance();
                    Token::RBracket
                }
                '{' => {
                    self.advance();
                    Token::LBrace
                }
                '}' => {
                    self.advance();
                    Token::RBrace
                }
                '"' => self.read_string(),
                '\'' => {
                    self.advance();
                    Token::Quote
                }
                ',' => {
                    self.advance();
                    if let Some(&ch) = self.peek() {
                        if ch.is_alphabetic() || ch == '_' {
                            let sym = self.read_symbol();
                            return Token::GlobalVar(sym);
                        }
                    }
                    Token::Comma
                }
                '.' => {
                    self.advance();
                    if let Some(&ch) = self.peek() {
                        if ch.is_alphabetic() || ch == '_' {
                            let sym = self.read_symbol();
                            return Token::LocalVar(sym);
                        }
                    }
                    Token::Dot
                }
                '#' => {
                    self.advance();
                    Token::Hash
                }
                '%' => {
                    self.advance();
                    Token::Percent
                }
                '^' => {
                    self.advance();
                    // Skip control characters like ^L (form feed)
                    if let Some(&ch) = self.peek() {
                        if ch.is_alphabetic() {
                            self.advance();
                        }
                    }
                    self.next_token()
                }
                '|' => {
                    self.advance();
                    Token::Pipe
                }
                '\\' => {
                    let sym = self.read_quoted_symbol();
                    Token::Symbol(sym)
                }
                '-' => {
                    // Check if it's a negative number or part of a symbol
                    self.advance();
                    if let Some(&next_ch) = self.peek() {
                        if next_ch.is_ascii_digit() {
                            let mut num = String::new();
                            while let Some(&ch) = self.peek() {
                                if ch.is_ascii_digit() {
                                    num.push(ch);
                                    self.advance();
                                } else {
                                    break;
                                }
                            }
                            let value: i64 = num.parse().unwrap_or(0);
                            Token::Number(-value)
                        } else {
                            // It's a minus sign as part of a symbol
                            let mut sym = String::from("-");
                            sym.push_str(&self.read_symbol());
                            Token::Symbol(sym)
                        }
                    } else {
                        Token::Symbol("-".to_string())
                    }
                }
                _ if ch.is_ascii_digit() => self.read_number(),
                _ if ch.is_alphabetic() || ch == '_' || ch == '$' => {
                    let sym = self.read_symbol();
                    Token::Symbol(sym)
                }
                _ => {
                    self.advance();
                    self.next_token()
                }
            },
        }
    }

    pub fn tokenize(&mut self) -> Vec<Token> {
        let mut tokens = Vec::new();
        loop {
            let token = self.next_token();
            if token == Token::EOF {
                tokens.push(token);
                break;
            }
            tokens.push(token);
        }
        tokens
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_tokens() {
        let mut lexer = Lexer::new("<ROOM WEST-OF-HOUSE>");
        assert_eq!(lexer.next_token(), Token::LParen);
        assert_eq!(lexer.next_token(), Token::Symbol("ROOM".to_string()));
        assert_eq!(lexer.next_token(), Token::Symbol("WEST-OF-HOUSE".to_string()));
        assert_eq!(lexer.next_token(), Token::RParen);
    }

    #[test]
    fn test_string() {
        let mut lexer = Lexer::new("\"Hello World\"");
        assert_eq!(lexer.next_token(), Token::String("Hello World".to_string()));
    }

    #[test]
    fn test_global_var() {
        let mut lexer = Lexer::new(",HERE");
        assert_eq!(lexer.next_token(), Token::GlobalVar("HERE".to_string()));
    }

    #[test]
    fn test_local_var() {
        let mut lexer = Lexer::new(".OBJ");
        assert_eq!(lexer.next_token(), Token::LocalVar("OBJ".to_string()));
    }
}
