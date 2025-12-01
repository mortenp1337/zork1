/// ZIL Abstract Syntax Tree
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub enum ZilValue {
    Nil,
    Number(i64),
    String(String),
    Symbol(String),
    Atom(String),
    List(Vec<ZilValue>),
    Table(Vec<ZilValue>),
    LocalVar(String),
    GlobalVar(String),
    Form(Vec<ZilValue>),     // <...>
    Segment(Vec<ZilValue>),  // Spliced list
    Quote(Box<ZilValue>),    // Quoted value
}

impl ZilValue {
    pub fn as_symbol(&self) -> Option<&str> {
        match self {
            ZilValue::Symbol(s) | ZilValue::Atom(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_string(&self) -> Option<&str> {
        match self {
            ZilValue::String(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_number(&self) -> Option<i64> {
        match self {
            ZilValue::Number(n) => Some(*n),
            _ => None,
        }
    }

    pub fn as_list(&self) -> Option<&Vec<ZilValue>> {
        match self {
            ZilValue::List(l) | ZilValue::Form(l) => Some(l),
            _ => None,
        }
    }

    pub fn is_nil(&self) -> bool {
        matches!(self, ZilValue::Nil)
    }

    pub fn to_bool(&self) -> bool {
        match self {
            ZilValue::Nil => false,
            ZilValue::Number(0) => false,
            ZilValue::List(l) if l.is_empty() => false,
            _ => true,
        }
    }
}

impl std::fmt::Display for ZilValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ZilValue::Nil => write!(f, "<>"),
            ZilValue::Number(n) => write!(f, "{}", n),
            ZilValue::String(s) => write!(f, "\"{}\"", s),
            ZilValue::Symbol(s) => write!(f, "{}", s),
            ZilValue::Atom(s) => write!(f, "{}", s),
            ZilValue::List(items) => {
                write!(f, "(")?;
                for (i, item) in items.iter().enumerate() {
                    if i > 0 {
                        write!(f, " ")?;
                    }
                    write!(f, "{}", item)?;
                }
                write!(f, ")")
            }
            ZilValue::Table(items) => {
                write!(f, "TABLE(")?;
                for (i, item) in items.iter().enumerate() {
                    if i > 0 {
                        write!(f, " ")?;
                    }
                    write!(f, "{}", item)?;
                }
                write!(f, ")")
            }
            ZilValue::LocalVar(s) => write!(f, ".{}", s),
            ZilValue::GlobalVar(s) => write!(f, ",{}", s),
            ZilValue::Form(items) => {
                write!(f, "<")?;
                for (i, item) in items.iter().enumerate() {
                    if i > 0 {
                        write!(f, " ")?;
                    }
                    write!(f, "{}", item)?;
                }
                write!(f, ">")
            }
            ZilValue::Segment(items) => {
                write!(f, "!<")?;
                for (i, item) in items.iter().enumerate() {
                    if i > 0 {
                        write!(f, " ")?;
                    }
                    write!(f, "{}", item)?;
                }
                write!(f, ">")
            }
            ZilValue::Quote(val) => write!(f, "'{}", val),
        }
    }
}

// ZIL object (room, item, etc.)
#[derive(Debug, Clone)]
pub struct ZilObject {
    pub name: String,
    pub properties: HashMap<String, ZilValue>,
    pub flags: Vec<String>,
    pub parent: Option<String>,
    pub first_child: Option<String>,
    pub sibling: Option<String>,
}

impl ZilObject {
    pub fn new(name: &str) -> Self {
        ZilObject {
            name: name.to_uppercase(),
            properties: HashMap::new(),
            flags: Vec::new(),
            parent: None,
            first_child: None,
            sibling: None,
        }
    }
}

// ZIL routine definition
#[derive(Debug, Clone)]
pub struct ZilRoutine {
    pub name: String,
    pub args: Vec<(String, bool)>,  // (name, is_optional)
    pub aux_vars: Vec<(String, Option<ZilValue>)>,  // (name, default_value)
    pub body: Vec<ZilValue>,
}

impl ZilRoutine {
    pub fn new(name: &str) -> Self {
        ZilRoutine {
            name: name.to_uppercase(),
            args: Vec::new(),
            aux_vars: Vec::new(),
            body: Vec::new(),
        }
    }
}

// ZIL syntax definition (verb patterns)
#[derive(Debug, Clone)]
pub struct ZilSyntax {
    pub verb: String,
    pub prep1: Option<String>,
    pub prep2: Option<String>,
    pub obj1_flags: Vec<String>,
    pub obj2_flags: Vec<String>,
    pub action: String,
    pub preaction: Option<String>,
}

// ZIL global variable
#[derive(Debug, Clone)]
pub struct ZilGlobal {
    pub name: String,
    pub value: ZilValue,
}

// ZIL constant
#[derive(Debug, Clone)]
pub struct ZilConstant {
    pub name: String,
    pub value: ZilValue,
}

// Top-level ZIL definition
#[derive(Debug, Clone)]
pub enum ZilDefinition {
    Object(ZilObject),
    Room(ZilObject),
    Routine(ZilRoutine),
    Syntax(ZilSyntax),
    Global(ZilGlobal),
    Constant(ZilConstant),
    Macro(ZilRoutine),
    Synonym(String, Vec<String>),
    Buzz(Vec<String>),
    Directions(Vec<String>),
    PropertyDefault(String, ZilValue),
    Version(String),
    InsertFile(String),
}
