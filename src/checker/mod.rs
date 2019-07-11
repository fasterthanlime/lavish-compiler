use crate::ast;
use std::fmt;

mod convos;
mod noredef;

mod print;
pub use print::print;

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
    // TODO: check name collisions in namespaces
    noredef::check(schema)?;
    convos::check(schema)?;
    Ok(())
}
