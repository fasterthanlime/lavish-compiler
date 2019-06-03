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
    fn qualified_name(&self) -> String;
    fn variant(&self) -> String;

    fn module(&self, stack: &ast::Stack) -> String;
    fn Params(&self, stack: &ast::Stack) -> String;
    fn Results(&self, stack: &ast::Stack) -> String;
    fn Client(&self, stack: &ast::Stack) -> String;
    fn Handler(&self, stack: &ast::Stack) -> String;
}

impl<'a> RustFn for ast::Anchored<'a, &ast::FunctionDecl> {
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

    fn module(&self, stack: &ast::Stack) -> String {
        format!(
            "{schema}::{path}",
            schema = stack.schema(),
            path = self.names().join("::")
        )
    }

    fn Params(&self, stack: &ast::Stack) -> String {
        format!("{module}::Params", module = self.module(stack))
    }
    fn Results(&self, stack: &ast::Stack) -> String {
        format!("{module}::Results", module = self.module(stack))
    }
    fn Client(&self, stack: &ast::Stack) -> String {
        format!("{module}::Client", module = self.module(stack))
    }
    fn Handler(&self, stack: &ast::Stack) -> String {
        format!("{module}::Handler", module = self.module(stack))
    }
}
