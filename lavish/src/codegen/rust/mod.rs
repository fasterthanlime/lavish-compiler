use super::super::ast;
use super::Error;
use heck::SnakeCase;
use indexmap::IndexMap;
use std::fs::File;
use std::time::Instant;

mod output;
use output::*;

struct Context<'a> {
    root: Namespace<'a>,
}

#[derive(PartialEq, Eq, Clone, Copy)]
enum FunKind {
    Request,
    Notification,
}

impl<'a> Context<'a> {
    fn new(root_body: &'a ast::NamespaceBody) -> Self {
        Self {
            root: Namespace::new("", "<root>", root_body),
        }
    }

    fn all_funs(&self) -> Box<Iterator<Item = &'a Fun> + 'a> {
        Box::new(self.root.funs())
    }

    fn funs(&self, kind: FunKind) -> Box<Iterator<Item = &'a Fun> + 'a> {
        let is_notification = kind == FunKind::Notification;

        Box::new(
            self.all_funs()
                .filter(move |x| x.is_notification() == is_notification),
        )
    }
}

struct Namespace<'a> {
    name: &'a str,

    children: IndexMap<&'a str, Namespace<'a>>,
    funs: IndexMap<&'a str, Fun<'a>>,
    strus: IndexMap<&'a str, Stru<'a>>,
}

impl<'a> Namespace<'a> {
    fn new(prefix: &str, name: &'a str, decl: &'a ast::NamespaceBody) -> Self {
        let prefix = if name == "<root>" {
            "".into()
        } else {
            format!("{}{}.", prefix, name)
        };

        let mut children: IndexMap<&'a str, Namespace<'a>> = IndexMap::new();
        let mut funs: IndexMap<&'a str, Fun<'a>> = IndexMap::new();
        let mut strus: IndexMap<&'a str, Stru<'a>> = IndexMap::new();

        for decl in &decl.functions {
            let full_name = format!("{}{}", prefix, decl.name.text);
            let ff = Fun::new(decl, full_name);
            funs.insert(&decl.name.text, ff);
        }

        for decl in &decl.structs {
            let full_name = format!("{}{}", prefix, decl.name.text);
            let st = Stru::new(decl, full_name);
            strus.insert(&decl.name.text, st);
        }

        for decl in &decl.namespaces {
            let name = decl.name.text.as_ref();
            children.insert(name, Namespace::new(&prefix, name, &decl.body));
        }

        Namespace {
            name,
            children,
            funs,
            strus,
        }
    }

    fn funs(&self) -> Box<Iterator<Item = &'a Fun> + 'a> {
        Box::new(
            self.children
                .values()
                .map(Namespace::funs)
                .flatten()
                .chain(self.funs.values()),
        )
    }

    fn name(&self) -> &'a str {
        self.name
    }
}

struct Fun<'a> {
    decl: &'a ast::FunctionDecl,
    tokens: Vec<String>,
}

impl<'a> Fun<'a> {
    fn new(decl: &'a ast::FunctionDecl, full_name: String) -> Self {
        Self {
            decl,
            tokens: full_name.split('.').map(|x| x.into()).collect(),
        }
    }

    fn rpc_name(&self) -> String {
        self.tokens.join(".")
    }

    fn variant_name(&self) -> String {
        self.rpc_name().replace(".", "_").to_lowercase()
    }

    fn qualified_name(&self) -> String {
        self.tokens.join("::")
    }

    fn mod_name(&self) -> String {
        self.decl.name.text.to_snake_case()
    }

    fn is_notification(&self) -> bool {
        self.decl
            .modifiers
            .contains(&ast::FunctionModifier::Notification)
    }

    fn has_empty_params(&self) -> bool {
        self.decl.params.is_empty()
    }

    fn has_empty_results(&self) -> bool {
        self.decl.results.is_empty()
    }
}

struct Stru<'a> {
    decl: &'a ast::StructDecl,
    #[allow(unused)]
    full_name: String,
}

impl<'a> Stru<'a> {
    fn new(decl: &'a ast::StructDecl, full_name: String) -> Self {
        Self { decl, full_name }
    }
}

pub type Result = std::result::Result<(), Error>;

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

fn visit_ns<'a>(s: &'a Scope<'a>, ns: &Namespace, depth: usize) -> Result {
    s.line(&format!("pub mod {} {{", ns.name()));
    {
        let s = s.scope();
        visit_ns_body(&s, ns, depth)?;
    }
    s.line("}"); // pub mod
    s.line("");
    Ok(())
}

fn visit_ns_body<'a>(s: &'a Scope<'a>, ns: &Namespace, depth: usize) -> Result {
    for (_, ns) in &ns.children {
        visit_ns(&s, ns, depth + 1)?;
    }

    s.line("use lavish_rpc::serde_derive::*;");
    s.line("");

    for (_, st) in &ns.strus {
        s.comment(&st.decl.comment);
        s.def_struct(&st.decl.name.text, &|s| {
            for f in &st.decl.fields {
                s.line(&format!("pub {}: {},", f.name.text, f.typ.as_rust()));
            }
        });
        s.line("");
    }

    for (_, fun) in &ns.funs {
        s.comment(&fun.decl.comment);
        s.line(&format!("pub mod {} {{", fun.mod_name()));

        {
            let s = s.scope();
            s.line("use futures::prelude::*;");
            s.line("use lavish_rpc::serde_derive::*;");
            let super_ref = "super::".repeat(depth + 2);
            s.line(&format!("use {}__;", super_ref));
            s.line("");

            let write_downgrade = |side: &str| {
                s.in_scope(&|s| {
                    s.line(&format!(
                        "pub fn downgrade(p: __::{}) -> Option<Self> {{",
                        side,
                    ));
                    s.in_scope(&|s| {
                        s.line("match p {");
                        s.in_scope(&|s| {
                            s.line(&format!(
                                "__::{}::{}(p) => Some(p),",
                                side,
                                fun.variant_name()
                            ));
                            s.line("_ => None,");
                        });
                        s.line("}"); // match p
                    });
                    s.line("}"); // fn downgrade
                });
            };

            s.def_struct("Params", &|s| {
                for f in &fun.decl.params {
                    s.line(&format!("pub {}: {},", f.name.text, f.typ.as_rust()));
                }
            });

            s.line("");
            s.line("impl Params {");
            write_downgrade(if fun.is_notification() {
                "NotificationParams"
            } else {
                "Params"
            });
            s.line("}"); // impl Params

            if !fun.is_notification() {
                s.line("");
                s.def_struct("Results", &|s| {
                    for f in &fun.decl.results {
                        s.line(&format!("pub {}: {},", f.name.text, f.typ.as_rust()));
                    }
                });

                s.line("");
                s.line("impl Results {");
                write_downgrade("Results");
                s.line("}"); // impl Results

                s.line("");
                let params_type = if fun.has_empty_params() {
                    "()"
                } else {
                    "Params"
                };
                s.line(&format!("pub async fn call(h: &__::Handle, p: {}) -> Result<Results, lavish_rpc::Error> {{", params_type));
                s.in_scope(&|s| {
                    s.line("h.call(");
                    s.in_scope(&|s| {
                        if fun.has_empty_params() {
                            s.line(&format!("__::Params::{}(Params {{}}),", fun.variant_name()));
                        } else {
                            s.line(&format!("__::Params::{}(p),", fun.variant_name()));
                        }
                        s.line("Results::downgrade,");
                    }); // h.call arguments
                    s.line(").await"); // h.call
                });
                s.line("}"); // async fn call

                s.line("");
                s.line("pub fn register<'a, T, F, FT>(h: &mut __::Handler<'a, T>, f: F)");
                s.line("where");
                let results_type = if fun.has_empty_results() {
                    "()"
                } else {
                    "Results"
                };
                s.in_scope(&|s| {
                    s.line("F: Fn(__::Call<T, Params>) -> FT + Sync + Send + 'a,");
                    s.line(&format!(
                        "FT: Future<Output = Result<{}, lavish_rpc::Error>> + Send + 'static,",
                        results_type
                    ));
                });
                s.line("{");
                s.in_scope(&|s| {
                    s.line(&format!(
                        "h.{} = Some(Box::new(move |state, handle, params| {{",
                        fun.variant_name(),
                    ));
                    s.in_scope(&|s| {
                        s.line("Box::pin(");
                        s.in_scope(&|s| {
                            s.line("f(__::Call {");
                            s.in_scope(&|s| {
                                s.line("state, handle,");
                                s.line("params: Params::downgrade(params).unwrap(),");
                            });
                            if fun.has_empty_results() {
                                s.line(&format!(
                                    "}}).map_ok(|_| __::Results::{}(Results {{}}))",
                                    fun.variant_name()
                                ));
                            } else {
                                s.line(&format!("}}).map_ok(__::Results::{})", fun.variant_name()));
                            }
                        });
                        s.line(")");
                    });
                    s.line("}));");
                });
                s.line("}"); // fn register
            }
        }
        s.line("}");
        s.line("");
    }
    Ok(())
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
            let output = Output::new(File::create(&mod_path).unwrap());
            let s = Scope::new(&output);
            self.write_prelude(&s);

            for member in workspace.members.values() {
                s.line(&format!("pub mod {};", member.name));
            }
        }

        Ok(())
    }
}

impl Generator {
    fn write_prelude<'a>(&self, s: &'a Scope<'a>) {
        s.line("// This file is generated by lavish: DO NOT EDIT");
        s.line("// https://github.com/fasterthanlime/lavish");
        s.line("");
        s.line("#![cfg_attr(rustfmt, rustfmt_skip)]");
        s.line("#![allow(clippy::all)]");
        s.line("#![allow(unknown_lints)]");
        s.line("#![allow(unused)]");
        s.line("");
    }

    fn emit(&self, workspace: &ast::Workspace, member: &ast::WorkspaceMember) -> Result {
        let start_instant = Instant::now();

        let output_path = workspace.dir.join(&member.name).join("mod.rs");
        std::fs::create_dir_all(output_path.parent().unwrap())?;
        let output = Output::new(File::create(&output_path).unwrap());

        let schema = member.schema.as_ref().unwrap();
        let ctx = Context::new(&schema.body);

        let s = Scope::new(&output);
        self.write_prelude(&s);

        fn write_enum<'a, I>(s: &Scope, kind: &str, funs: I)
        where
            I: Iterator<Item = &'a Fun<'a>>,
        {
            let s = s.scope();
            for fun in funs {
                s.line(&format!(
                    "{}({}::{}),",
                    fun.variant_name(),
                    fun.qualified_name(),
                    kind,
                ));
            }
        };

        {
            s.line("pub use __::*;");
            s.line("");
            s.line("mod __ {");
            let s = s.scope();

            s.line("// Notes: as of 2019-05-21, futures-preview is required");
            s.line("use futures::prelude::*;");
            s.line("use std::pin::Pin;");
            s.line("use std::sync::Arc;");
            s.line("");
            s.line("use lavish_rpc as rpc;");
            s.line("use rpc::{Atom, erased_serde, serde_derive::*};");

            s.line("");
            s.line("#[derive(Serialize, Debug)]");
            s.line("#[serde(untagged)]");
            s.line("#[allow(non_camel_case_types, unused)]");
            s.line("pub enum Params {");
            write_enum(&s, "Params", ctx.funs(FunKind::Request));
            s.line("}"); // enum Params

            s.line("");
            s.line("#[derive(Serialize, Debug)]");
            s.line("#[serde(untagged)]");
            s.line("#[allow(non_camel_case_types, unused)]");
            s.line("pub enum Results {");
            write_enum(&s, "Results", ctx.funs(FunKind::Request));
            s.line("}"); // enum Results

            s.line("");
            s.line("#[derive(Serialize, Debug)]");
            s.line("#[serde(untagged)]");
            s.line("#[allow(non_camel_case_types, unused)]");
            s.line("pub enum NotificationParams {");
            write_enum(&s, "Params", ctx.funs(FunKind::Notification));
            s.line("}"); // enum NotificationParams

            s.line("");
            s.line("pub type Message = rpc::Message<Params, NotificationParams, Results>;");
            s.line("pub type Handle = rpc::Handle<Params, NotificationParams, Results>;");
            s.line("pub type System = rpc::System<Params, NotificationParams, Results>;");
            s.line("pub type Protocol = rpc::Protocol<Params, NotificationParams, Results>;");
            s.line("");
            s.line("pub fn protocol() -> Protocol {");
            s.in_scope(&|s| {
                s.line("Protocol::new()");
            });
            s.line("}"); // fn protocol

            for (strukt, side, kind) in &[
                ("Params", "Params", FunKind::Request),
                ("Results", "Results", FunKind::Request),
                ("Params", "NotificationParams", FunKind::Notification),
            ] {
                s.line("");
                s.line(&format!("impl rpc::Atom for {} {{", side));
                s.in_scope(&|s| {
                    s.line("fn method(&self) -> &'static str {");
                    s.in_scope(&|s| {
                        s.line("match self {");
                        s.in_scope(&|s| {
                            let mut count = 0;
                            for fun in ctx.funs(*kind) {
                                count += 1;
                                s.line(&format!(
                                    "{}::{}(_) => {:?},",
                                    side,
                                    fun.variant_name(),
                                    fun.rpc_name()
                                ));
                            }
                            if count == 0 {
                                s.line("_ => unimplemented!()")
                            }
                        });
                        s.line("}");
                    });
                    s.line("}"); // fn method

                    s.line("");
                    s.line("fn deserialize(");
                    s.in_scope(&|s| {
                        s.line("method: &str,");
                        s.line("de: &mut erased_serde::Deserializer,");
                    });
                    s.line(") -> erased_serde::Result<Self> {");
                    s.in_scope(&|s| {
                        s.line("use erased_serde::deserialize as deser;");
                        s.line("use serde::de::Error;");
                        s.line("");
                        s.line("match method {");
                        s.in_scope(&|s| {
                            for fun in ctx.funs(*kind) {
                                s.line(&format!("{:?} =>", fun.rpc_name(),));
                                {
                                    let s = s.scope();
                                    s.line(&format!(
                                        "Ok({}::{}(deser::<{}::{}>(de)?)),",
                                        side,
                                        fun.variant_name(),
                                        fun.qualified_name(),
                                        strukt,
                                    ));
                                }
                            }
                            s.line("_ => Err(erased_serde::Error::custom(format!(");
                            s.in_scope(&|s| {
                                s.line(&format!("{:?},", "unknown method: {}"));
                                s.line("method,");
                            });
                            s.line("))),");
                        });
                        s.line("}");
                    });
                    s.line("}"); // fn deserialize
                });
                s.line("}"); // impl Atom for side
            } // impl rpc::Atom for P, NP, R

            s.line("");
            s.line("pub struct Call<T, PP> {");
            s.in_scope(&|s| {
                s.line("pub state: Arc<T>,");
                s.line("pub handle: Handle,");
                s.line("pub params: PP,");
            });
            s.line("}"); // struct Call

            s.line("");
            s.line("pub type SlotFuture = ");
            s.in_scope(&|s| {
                s.line("Future<Output = Result<Results, rpc::Error>> + Send + 'static;");
            });

            s.line("");
            s.line("pub type SlotReturn = Pin<Box<SlotFuture>>;");

            s.line("");
            s.line("pub type SlotFn<'a, T> = ");
            s.in_scope(&|s| {
                s.line("Fn(Arc<T>, Handle, Params) -> SlotReturn + 'a + Send + Sync;");
            });

            s.line("");
            s.line("pub type Slot<'a, T> = Option<Box<SlotFn<'a, T>>>;");

            s.line("");
            s.line("pub struct Handler<'a, T> {");
            s.in_scope(&|s| {
                s.line("state: Arc<T>,");
                for fun in ctx.funs(FunKind::Request) {
                    s.line(&format!("{}: Slot<'a, T>,", fun.variant_name()));
                }
            });
            s.line("}"); // struct Handler

            s.line("");
            s.line("impl<'a, T> Handler<'a, T> {");
            s.in_scope(&|s| {
                s.line("pub fn new(state: T) -> Self {");
                s.in_scope(&|s| {
                    s.line("Self {");
                    s.in_scope(&|s| {
                        s.line("state: Arc::new(state),");
                        for fun in ctx.funs(FunKind::Request) {
                            s.line(&format!("{}: None,", fun.variant_name()));
                        }
                    });
                    s.line("}");
                });
                s.line("}");
            });
            s.line("}"); // impl Handler

            s.line("");
            s.line("type HandlerRet = Pin<Box<dyn Future<Output = Result<Results, rpc::Error>> + Send + 'static>>;");
            s.line("");
            s.line("impl<'a, T> rpc::Handler<Params, NotificationParams, Results, HandlerRet> for Handler<'a, T>");
            s.line("where");
            s.in_scope(&|s| {
                s.line("T: Send + Sync,");
            });
            s.line("{");
            s.in_scope(&|s| {
            s.line("fn handle(&self, handle: Handle, params: Params) -> HandlerRet {");
            s.in_scope(&|s| {
                s.line("let method = params.method();");
                s.line("let slot = match params {");
                s.in_scope(&|s| {
                    for fun in ctx.funs(FunKind::Request) {
                        s.line(&format!(
                            "Params::{}(_) => self.{}.as_ref(),",
                            fun.variant_name(),
                            fun.variant_name()
                        ));
                    }
                    s.line("_ => None,");
                });
                s.line("};");
                s.line("match slot {");
                s.in_scope(&|s| {
                    s.line("Some(slot_fn) => {");
                    s.in_scope(&|s| {
                        s.line("let res = slot_fn(self.state.clone(), handle, params);");
                        s.line("Box::pin(async move { Ok(res.await?) })");
                    });
                    s.line("}"); // Some(slot_fn)
                    s.line("None => Box::pin(async move { Err(rpc::Error::MethodUnimplemented(method)) }),");
                });
                s.line("}"); // match slot
            });
            s.line("}");
        });
            s.line("}"); // impl rpc::Handler for Handler

            s.line("");
            visit_ns_body(&s, &ctx.root, 0)?;
        }
        s.line("}"); // mod __root

        let end_instant = Instant::now();
        println!(
            "Generated {:?} in {:?}",
            output_path,
            end_instant.duration_since(start_instant)
        );

        Ok(())
    }
}
