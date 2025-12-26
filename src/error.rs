use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// Maximum interpolation depth exceeded (potential cycle)
    RecursiveLookup(String),
    /// Variable not found
    MissingVar(String),
    /// Syntax error at position
    SyntaxError(String, usize),
    /// Unterminated variable brace
    UnclosedBrace(usize),
    /// Unterminated quote
    UnclosedQuote(usize),
    /// Command execution error
    CommandError(String),
    /// IO Error
    IoError(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::RecursiveLookup(ctx) => write!(f, "Maximum interpolation depth exceeded: {}", ctx),
            Error::MissingVar(var) => write!(f, "Variable not found: {}", var),
            Error::SyntaxError(msg, pos) => write!(f, "Syntax error at position {}: {}", pos, msg),
            Error::UnclosedBrace(pos) => write!(f, "Unclosed variable brace starting at position {}", pos),
            Error::UnclosedQuote(pos) => write!(f, "Unterminated quote starting at position {}", pos),
            Error::CommandError(msg) => write!(f, "Command execution failed: {}", msg),
            Error::IoError(msg) => write!(f, "IO Error: {}", msg),
        }
    }
}

impl std::error::Error for Error {}
