
mod rust;

pub enum Target {
    Rust
}

pub fn codegen(module: &ast::Module, target: Target) -> Result<(), Error> {
    match target {
        Rust => rust::codegen(module)
    }
}
