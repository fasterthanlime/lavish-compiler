use super::ast;
use super::parser;

mod noredef;

pub struct Error {
    pub num_errors: i64,
}

pub fn check(module: &ast::Module) -> Result<(), Error> {
    noredef::check(module)?;
    Ok(())
}
