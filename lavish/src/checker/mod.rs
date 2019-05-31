use super::ast;

mod convos;
mod noredef;
use std::fmt;

#[derive(Debug)]
pub struct Error {
    pub num_errors: i64,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{} errors found", self.num_errors)
    }
}

impl std::error::Error for Error {}

pub fn check(schema: &ast::Schema) -> Result<(), Error> {
    noredef::check(schema)?;
    convos::check(schema)?;
    Ok(())
}
