use crate::ast;
use crate::codegen::output::*;
use std::fmt::{self, Display, Write};

pub struct Symbols<'a> {
    body: ast::Anchored<'a, &'a ast::NamespaceBody>,
}

impl<'a> Symbols<'a> {
    pub fn new(body: ast::Anchored<'a, &'a ast::NamespaceBody>) -> Self {
        Self { body }
    }
}

impl<'a> Display for Symbols<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        Scope::fmt(f, |s| {
            let body = &self.body;

            s.line(format!("// trace = {}", body.stack.trace()));

            for ns in &body.inner.namespaces {
                let stack = body.stack.push(ns);
                write!(s, "pub mod {}", ns.name.text).unwrap();
                s.in_block(|s| {
                    s.write(Symbols::new(stack.anchor(&ns.body)));
                });
            }
        })
    }
}
