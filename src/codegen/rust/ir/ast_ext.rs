#![allow(non_snake_case)]

use crate::ast;
use heck::CamelCase;

pub trait RustStack {
    fn root(&self) -> String;
    fn protocol(&self) -> String;
    fn schema(&self) -> String;
    fn RootClient(&self) -> String;
}

impl<'a> RustStack for ast::Stack<'a> {
    fn root(&self) -> String {
        "super::".repeat(self.frames.len() + 1)
    }

    fn protocol(&self) -> String {
        format!("{}protocol", self.root())
    }

    fn schema(&self) -> String {
        format!("{}schema", self.root())
    }

    fn RootClient(&self) -> String {
        format!("{}::Client", self.protocol())
    }
}

pub trait RustFn {
    fn module(&self) -> String;
    fn qualified_name(&self) -> String;
    fn variant(&self) -> String;
}

impl<'a> RustFn for ast::Anchored<'a, &ast::FunctionDecl> {
    fn module(&self) -> String {
        self.names().join("::")
    }

    fn variant(&self) -> String {
        self.names()
            .iter()
            .map(|x| x.to_camel_case())
            .collect::<Vec<_>>()
            .join("_")
    }

    fn qualified_name(&self) -> String {
        self.names().join("__")
    }
}
