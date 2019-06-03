use crate::codegen::rust::prelude::*;

pub trait AsRust {
    fn as_rust<'a>(&'a self, stack: &'a ast::Stack<'a>) -> Box<fmt::Display + 'a>;
}

struct RustType<'a>(pub ast::Anchored<'a, &'a ast::Type>);

impl AsRust for ast::Type {
    fn as_rust<'a>(&'a self, stack: &'a ast::Stack<'a>) -> Box<fmt::Display + 'a> {
        Box::new(RustType(stack.anchor(self)))
    }
}

use std::fmt;
impl<'a> fmt::Display for RustType<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use ast::{BaseType, TypeKind};

        match &self.0.kind {
            TypeKind::Base(base) => {
                let name = match base {
                    BaseType::Bool => "bool",
                    BaseType::Int32 => "i32",
                    BaseType::Int64 => "i64",
                    BaseType::UInt32 => "u32",
                    BaseType::UInt64 => "u64",
                    BaseType::Float32 => "f32",
                    BaseType::Float64 => "f64",
                    BaseType::String => "String",
                    BaseType::Bytes => "Vec<u8>",
                    BaseType::Timestamp => "::lavish_rpc::DateTime",
                };
                write!(f, "{}", name)
            }
            TypeKind::Map(map) => write!(
                f,
                "::std::collections::HashMap<{}, {}>",
                map.keys.as_rust(&self.0.stack),
                map.values.as_rust(&self.0.stack)
            ),
            TypeKind::Option(opt) => write!(f, "Option<{}>", opt.inner.as_rust(&self.0.stack)),
            TypeKind::Array(arr) => write!(f, "Vec<{}>", arr.inner.as_rust(&self.0.stack)),
            TypeKind::User => {
                // TODO: actually resolve those
                write!(f, "super::{}", self.0.text())
            }
        }
    }
}
