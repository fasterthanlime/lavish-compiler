#![allow(non_snake_case)]

use crate::ast;
use heck::CamelCase;

pub trait RustStack {
    fn root(&self) -> String;
    fn protocol(&self) -> String;
    fn schema(&self) -> String;
    fn Caller(&self) -> String;
    fn Callable(&self) -> String;
    fn Implementable(&self) -> String;
    fn TranslationTables(&self) -> String;
    fn Slottable(&self) -> String;
    fn SideClient(&self, side: ast::Side) -> String;
    fn Params(&self) -> String;
    fn NotificationParams(&self) -> String;
    fn Results(&self) -> String;
    fn triplet(&self) -> String;
}

impl<'a> RustStack for ast::Stack<'a> {
    fn root(&self) -> String {
        "super::".repeat(self.frames.len())
    }

    fn protocol(&self) -> String {
        format!("{}protocol", self.root())
    }

    fn schema(&self) -> String {
        format!("{}schema", self.root())
    }

    fn Caller(&self) -> String {
        format!("{}::Caller", self.protocol())
    }

    fn Callable(&self) -> String {
        format!("{}::Callable", self.protocol())
    }

    fn Implementable(&self) -> String {
        format!("{}::Implementable", self.protocol())
    }

    fn TranslationTables(&self) -> String {
        format!("{}::TranslationTables", self.protocol())
    }

    fn Slottable(&self) -> String {
        format!("{}::Slottable", self.protocol())
    }

    fn SideClient(&self, side: ast::Side) -> String {
        format!("super::{}::Client", side)
    }

    fn Params(&self) -> String {
        format!("{}::Params", self.protocol())
    }

    fn NotificationParams(&self) -> String {
        format!("{}::NotificationParams", self.protocol())
    }

    fn Results(&self) -> String {
        format!("{}::Results", self.protocol())
    }

    fn triplet(&self) -> String {
        format!(
            "{P}, {NP}, {R}",
            P = self.Params(),
            NP = self.NotificationParams(),
            R = self.Results(),
        )
    }
}

pub trait RustFn {
    fn slot(&self) -> String;
    fn rust_name(&self) -> String;
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

    // Name of the slot in a Handler, for example `session.attempt_login`
    // will have slot name `on_session__attempt_login`
    fn slot(&self) -> String {
        format!("on_{}", self.rust_name())
    }

    // Rust name of a function, for example `session.attempt_login`
    // will have name `session__attempt_login`
    fn rust_name(&self) -> String {
        format!("{}", self.names().join("__"))
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

pub trait RustStruct {
    fn variant(&self) -> String;
}

impl<'a> RustStruct for ast::Anchored<'a, &ast::StructDecl> {
    fn variant(&self) -> String {
        self.names()
            .iter()
            .map(|x| x.to_camel_case())
            .collect::<Vec<_>>()
            .join("_")
    }
}
