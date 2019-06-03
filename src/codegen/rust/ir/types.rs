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
        use ast::TypeKind;

        match &self.0.kind {
            TypeKind::Base(base) => base.write_rust_type(f),
            TypeKind::Map(map) => write!(
                f,
                "{collections}::HashMap<{K}, {V}>",
                collections = Mods::collections(),
                K = map.keys.as_rust(&self.0.stack),
                V = map.values.as_rust(&self.0.stack)
            ),
            TypeKind::Option(opt) => write!(f, "Option<{T}>", T = opt.inner.as_rust(&self.0.stack)),
            TypeKind::Array(arr) => write!(f, "Vec<{T}>", T = arr.inner.as_rust(&self.0.stack)),
            TypeKind::User => {
                // TODO: actually resolve those, using our stack
                write!(f, "super::{}", self.0.text())
            }
        }
    }
}

trait RustBaseType {
    fn write_rust_type(&self, f: &mut fmt::Formatter) -> fmt::Result;
}

impl RustBaseType for ast::BaseType {
    fn write_rust_type(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use ast::BaseType as T;

        match self {
            T::Bool => write!(f, "bool"),
            T::Int32 => write!(f, "i32"),
            T::Int64 => write!(f, "i64"),
            T::UInt32 => write!(f, "u32"),
            T::UInt64 => write!(f, "u64"),
            T::Float32 => write!(f, "f32"),
            T::Float64 => write!(f, "f64"),
            T::String => write!(f, "String"),
            T::Bytes => write!(f, "Vec<u8>"),
            T::Timestamp => write!(f, "{chrono}::DateTime", chrono = Mods::chrono()),
        }
    }
}
