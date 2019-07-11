use crate::codegen::rust::prelude::*;
use colored::*;

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
            TypeKind::Base(base) => base.generate_rust(f),
            TypeKind::Map(map) => write!(
                f,
                "{HashMap}<{K}, {V}>",
                HashMap = Structs::HashMap(),
                K = map.keys.as_rust(&self.0.stack),
                V = map.values.as_rust(&self.0.stack)
            ),
            TypeKind::Option(opt) => write!(f, "Option<{T}>", T = opt.inner.as_rust(&self.0.stack)),
            TypeKind::Array(arr) => write!(f, "Vec<{T}>", T = arr.inner.as_rust(&self.0.stack)),
            TypeKind::User => {
                let t = &self.0;
                let down: Vec<_> = t.text().split(".").collect();
                match t.stack.lookup_struct(ast::LookupMode::Relaxed, &down[..]) {
                    Some(path) => path.generate_rust(f),
                    None => {
                        t.span
                            .position()
                            .diag_err(format!(
                                "{} unknown type {:?}: not a built-in, and not in scope either",
                                "error:".red().bold(),
                                t.text(),
                            ))
                            .print();
                        panic!("Failed to resolve type");
                    }
                }
            }
        }
    }
}

trait GeneratesRust {
    fn generate_rust(&self, f: &mut fmt::Formatter) -> fmt::Result;
}

impl GeneratesRust for ast::BaseType {
    fn generate_rust(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use ast::BaseType as T;

        match self {
            T::Bool => write!(f, "bool"),
            T::I8 => write!(f, "i8"),
            T::I16 => write!(f, "i16"),
            T::I32 => write!(f, "i32"),
            T::I64 => write!(f, "i64"),
            T::U8 => write!(f, "u8"),
            T::U16 => write!(f, "u16"),
            T::U32 => write!(f, "u32"),
            T::U64 => write!(f, "u64"),
            T::F32 => write!(f, "f32"),
            T::F64 => write!(f, "f64"),
            T::String => write!(f, "String"),
            T::Data => write!(f, "{facts}::Bin", facts = Mods::facts()),
            T::Timestamp => write!(
                f,
                "{chrono}::DateTime<{chrono}::offset::Utc>",
                chrono = Mods::chrono()
            ),
        }
    }
}

impl<'a> GeneratesRust for ast::RelativePath<'a> {
    fn generate_rust(&self, f: &mut fmt::Formatter) -> fmt::Result {
        Scope::fmt(f, |s| {
            let mut list = List::new(s, "::", Brackets::None);
            for _ in 0..self.up {
                list.item("super");
            }
            for item in &self.down {
                list.item(item);
            }
        })
    }
}
