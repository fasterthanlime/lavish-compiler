pub(crate) use crate::codegen::{
    prelude::*,
    rust::ir::{common::*, lang::*, types::*},
};

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
