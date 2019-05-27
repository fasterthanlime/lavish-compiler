mod rust;
use super::ast;

use std::fmt;

#[derive(Debug)]
pub struct Error {
    message: String,
}

pub type Result = std::result::Result<(), Error>;

pub trait Generator {
    fn emit(&self, workspace: &ast::Workspace, member: &ast::WorkspaceMember) -> Result;
}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Self {
            message: format!("i/o error: {}", e),
        }
    }
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

pub fn codegen(workspace: &ast::Workspace) -> Result {
    let generator = match &workspace.rules.target {
        ast::Target::Rust(target) => rust::Generator::new(target.clone()),
        _ => panic!("Unimplemented target: {:#?}", workspace.rules.target),
    };

    for member in workspace.members.values() {
        generator.emit(&workspace, member)?;
    }

    println!("Codegen done!");
    Ok(())
}
