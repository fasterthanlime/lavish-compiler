mod rust;
use super::ast;

use std::fmt;

pub enum Target {
    Rust,
}

#[derive(Debug)]
pub struct Error {
    message: String,
}

impl From<&'static str> for Error {
    fn from(s: &'static str) -> Self {
        Self { message: s.into() }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "codegen error: {}", self.message)
    }
}

pub fn codegen(modules: &Vec<ast::Module>, target: Target) -> Result<(), Error> {
    match target {
        Target::Rust => rust::codegen(modules),
    }
}
