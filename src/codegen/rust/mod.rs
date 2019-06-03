use crate::ast;
use crate::codegen::output::*;
use crate::codegen::Result;

use std::fmt::Write;
use std::fs::File;
use std::time::Instant;

mod ir;
use ir::*;

trait AsRust {
    fn as_rust<'a>(&'a self) -> Box<fmt::Display + 'a>;
}

struct RustType<'a>(pub &'a ast::Type);

impl AsRust for ast::Type {
    fn as_rust<'a>(&'a self) -> Box<fmt::Display + 'a> {
        Box::new(RustType(self))
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
                map.keys.as_rust(),
                map.values.as_rust()
            ),
            TypeKind::Option(opt) => write!(f, "Option<{}>", opt.inner.as_rust()),
            TypeKind::Array(arr) => write!(f, "Vec<{}>", arr.inner.as_rust()),
            TypeKind::User => {
                // TODO: actually resolve those
                write!(f, "super::{}", self.0.text())
            }
        }
    }
}

pub struct Generator {
    #[allow(unused)]
    target: ast::RustTarget,
}

impl Generator {
    pub fn new(target: ast::RustTarget) -> Self {
        Self { target }
    }
}

impl super::Generator for Generator {
    fn emit_workspace(&self, workspace: &ast::Workspace) -> Result {
        for member in workspace.members.values() {
            self.emit(workspace, member)?;
        }

        {
            let mod_path = workspace.dir.join("mod.rs");
            let mut output = Scope::writer(File::create(&mod_path)?);
            let mut s = Scope::new(&mut output);
            self.write_prelude(&mut s);

            for member in workspace.members.values() {
                writeln!(s, "pub mod {};", member.name)?;
            }
        }

        Ok(())
    }
}

impl Generator {
    fn write_prelude<'a>(&self, s: &mut Scope<'a>) {
        s.line("// This file is generated by lavish: DO NOT EDIT");
        s.line("// https://github.com/fasterthanlime/lavish");
        s.lf();
        s.line("#![cfg_attr(rustfmt, rustfmt_skip)]");
        s.line("#![allow(clippy::all, unknown_lints, unused, non_snake_case)]");
        s.lf();
    }

    fn emit(&self, workspace: &ast::Workspace, member: &ast::WorkspaceMember) -> Result {
        let start_instant = Instant::now();

        let output_path = workspace.dir.join(&member.name).join("mod.rs");
        std::fs::create_dir_all(output_path.parent().unwrap())?;
        let mut output = Scope::writer(File::create(&output_path)?);
        let mut s = Scope::new(&mut output);
        self.write_prelude(&mut s);

        let schema = member.schema.as_ref().expect("schema to be parsed");
        let stack = ast::Stack::new();
        let body = stack.anchor(&schema.body);

        {
            s.write(Protocol { body: body.clone() });
            s.lf();
        }

        {
            write!(s, "pub mod schema").unwrap();
            s.in_block(|s| {
                s.write(Symbols::new(body.clone()));
            });
            s.lf();
        }

        {
            {
                write!(s, "pub mod client").unwrap();
                s.in_block(|s| {
                    s.write(Client {
                        body: body.clone(),
                        side: ast::Side::Client,
                    });
                });
            }
            s.lf();
            {
                write!(s, "pub mod server").unwrap();
                s.in_block(|s| {
                    s.write(Client {
                        body: body.clone(),
                        side: ast::Side::Server,
                    });
                });
            }
        }

        let end_instant = Instant::now();
        println!(
            "Generated {:?} in {:?}",
            output_path,
            end_instant.duration_since(start_instant)
        );

        Ok(())
    }
}
