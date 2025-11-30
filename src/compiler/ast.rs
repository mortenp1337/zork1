//! ZIL Abstract Syntax Tree
//!
//! Defines the AST nodes for ZIL language constructs.

use std::collections::HashMap;

/// A ZIL expression (S-expression)
#[derive(Debug, Clone)]
pub enum Expr {
    /// Atom (symbol/identifier)
    Atom(String),
    /// Integer literal
    Number(i32),
    /// String literal
    String(String),
    /// List (form or data list)
    List(Vec<Expr>),
    /// Quoted expression
    Quote(Box<Expr>),
    /// Global value reference (,var)
    GVal(String),
    /// Local value reference (.var)
    LVal(String),
    /// Table literal
    Table(Vec<Expr>),
    /// Pure table (PTABLE)
    PTable(Vec<Expr>),
    /// Length-prefixed table (LTABLE)
    LTable(Vec<Expr>),
    /// Byte table (ITABLE)
    ITable(usize, Option<Box<Expr>>),
    /// Form (function call) '<...>
    Form(String, Vec<Expr>),
    /// Boolean false <>
    False,
    /// Boolean true (T)
    True,
}

/// Top-level ZIL forms
#[derive(Debug, Clone)]
pub enum Form {
    /// <CONSTANT name value>
    Constant(String, Expr),
    /// <GLOBAL name value>
    Global(String, Expr),
    /// <OBJECT ...>
    Object(ObjectDef),
    /// <ROOM ...>
    Room(RoomDef),
    /// <ROUTINE name (args) body...>
    Routine(RoutineDef),
    /// <SYNTAX ...>
    Syntax(SyntaxDef),
    /// <VERB-SYNONYM ...>
    VerbSynonym(String, Vec<String>),
    /// <INSERT-FILE filename flag?>
    InsertFile(String, bool),
    /// <DEFMAC name (args) body>
    DefMacro(String, Vec<String>, Vec<Expr>),
    /// <SETG name value>
    SetG(String, Expr),
    /// <VERSION ver>
    Version(String),
    /// <DIRECTIONS dir...>
    Directions(Vec<String>),
    /// <PROPDEF name default>
    PropDef(String, i32),
    /// <FREQUENT-WORDS?>
    FrequentWords,
    /// <BUZZ words...>
    Buzz(Vec<String>),
    /// Other forms we don't need to process specially
    Other(String, Vec<Expr>),
}

/// Object definition
#[derive(Debug, Clone)]
pub struct ObjectDef {
    pub name: String,
    pub properties: HashMap<String, Property>,
}

/// Room definition (similar to object but for locations)
#[derive(Debug, Clone)]
pub struct RoomDef {
    pub name: String,
    pub properties: HashMap<String, Property>,
}

/// Object/Room property
#[derive(Debug, Clone)]
pub enum Property {
    /// String property
    String(String),
    /// Number property
    Number(i32),
    /// Symbol reference
    Symbol(String),
    /// List of values
    List(Vec<Expr>),
    /// Exit/direction property (for rooms)
    Exit(ExitDef),
    /// Flags
    Flags(Vec<String>),
    /// Action routine
    Action(String),
    /// Description function
    DescFcn(String),
    /// Synonyms
    Synonym(Vec<String>),
    /// Adjectives
    Adjective(Vec<String>),
    /// Pseudo objects
    Pseudo(Vec<(String, String)>),
    /// Container property
    In(String),
    /// Generic expression
    Expr(Expr),
}

/// Exit definition for rooms
#[derive(Debug, Clone)]
pub struct ExitDef {
    pub direction: String,
    pub destination: ExitDestination,
}

/// Exit destination
#[derive(Debug, Clone)]
pub enum ExitDestination {
    /// Direct room reference
    Room(String),
    /// Conditional exit (via routine)
    Conditional(String),
    /// Unconditional exit with message
    Message(String),
    /// No exit with special handling
    Special(String, String),
}

/// Routine definition
#[derive(Debug, Clone)]
pub struct RoutineDef {
    pub name: String,
    pub args: Vec<ArgDef>,
    pub body: Vec<Expr>,
}

/// Argument definition
#[derive(Debug, Clone)]
pub struct ArgDef {
    pub name: String,
    pub arg_type: ArgType,
    pub default: Option<Expr>,
}

/// Argument type
#[derive(Debug, Clone, PartialEq)]
pub enum ArgType {
    /// Required argument
    Required,
    /// Optional argument
    Optional,
    /// Auxiliary (local) variable
    Aux,
    /// Rest arguments (collect remaining)
    Args,
}

/// Syntax definition for verbs
#[derive(Debug, Clone)]
pub struct SyntaxDef {
    pub verb: String,
    pub patterns: Vec<SyntaxPattern>,
}

/// Syntax pattern
#[derive(Debug, Clone)]
pub struct SyntaxPattern {
    pub words: Vec<String>,
    pub objects: Vec<SyntaxObject>,
    pub action: String,
    pub preaction: Option<String>,
}

/// Object in syntax pattern
#[derive(Debug, Clone)]
pub struct SyntaxObject {
    pub finder: String,
    pub flags: Vec<String>,
}

impl ObjectDef {
    pub fn new(name: String) -> Self {
        ObjectDef {
            name,
            properties: HashMap::new(),
        }
    }
}

impl RoomDef {
    pub fn new(name: String) -> Self {
        RoomDef {
            name,
            properties: HashMap::new(),
        }
    }
}

impl RoutineDef {
    pub fn new(name: String) -> Self {
        RoutineDef {
            name,
            args: Vec::new(),
            body: Vec::new(),
        }
    }
}

impl Expr {
    /// Check if expression is an atom with given name
    pub fn is_atom(&self, name: &str) -> bool {
        matches!(self, Expr::Atom(s) if s.eq_ignore_ascii_case(name))
    }
    
    /// Get atom name if this is an atom
    pub fn as_atom(&self) -> Option<&str> {
        match self {
            Expr::Atom(s) => Some(s),
            _ => None,
        }
    }
    
    /// Get number if this is a number
    pub fn as_number(&self) -> Option<i32> {
        match self {
            Expr::Number(n) => Some(*n),
            _ => None,
        }
    }
    
    /// Get string if this is a string
    pub fn as_string(&self) -> Option<&str> {
        match self {
            Expr::String(s) => Some(s),
            _ => None,
        }
    }
    
    /// Get list if this is a list
    pub fn as_list(&self) -> Option<&[Expr]> {
        match self {
            Expr::List(l) => Some(l),
            _ => None,
        }
    }
}
