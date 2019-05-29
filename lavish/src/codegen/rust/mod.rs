use crate::ast;
use crate::codegen::Error;
use std::fs::File;
use std::time::Instant;

mod ir;
use ir::*;

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

fn visit_ns<'a>(s: &mut Scope<'a>, ns: &Namespace, depth: usize) -> Result {
    s.line(format!("pub mod {} {{", ns.name()));
    {
        let mut s = s.scope();
        visit_ns_body(&mut s, ns, depth)?;
    }
    s.line("}"); // pub mod
    s.line("");
    Ok(())
}

fn visit_ns_body<'a>(s: &mut Scope<'a>, ns: &'a Namespace<'a>, depth: usize) -> Result {
    for (_, ns) in ns.children() {
        visit_ns(s, ns, depth + 1)?;
    }

    s.line("use lavish_rpc::serde_derive::*;");
    s.line("");

    for (_, st) in ns.strus() {
        s.comment(&st.comment());
        s.def_struct(st.name(), |s| {
            for f in st.fields() {
                s.comment(&f.comment);
                s.line(format!("pub {}: {},", f.name.text, f.typ.as_rust()));
            }
        });
        s.line("");
    }

    let write_fun = |s: &mut Scope, fun: &Fun<'a>| -> Result {
        s.comment(fun.comment());
        s.line(format!("pub mod {} {{", fun.mod_name()));

        {
            let mut s = s.scope();
            s.line("use futures::prelude::*;");
            s.line("use lavish_rpc::serde_derive::*;");
            let super_ref = "super::".repeat(depth + 2);
            s.line(format!("use {}__;", super_ref));
            s.line("");

            let write_downgrade = |s: &mut Scope, side: &str| {
                s.in_scope(|s| {
                    s.line(format!(
                        "pub fn downgrade(p: __::{}) -> Option<Self> {{",
                        side,
                    ));
                    s.in_scope(|s| {
                        s.line("match p {");
                        s.in_scope(|s| {
                            s.line(format!(
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

            s.def_struct("Params", |s| {
                for f in fun.params().fields() {
                    s.line(format!("pub {}: {},", f.name.text, f.typ.as_rust()));
                }
            });

            s.line("");
            s.line("impl Params {");
            write_downgrade(
                &mut s,
                if fun.is_notification() {
                    "NotificationParams"
                } else {
                    "Params"
                },
            );
            s.line("}"); // impl Params

            if !fun.is_notification() {
                s.line("");
                s.def_struct("Results", |s| {
                    for f in fun.results().fields() {
                        s.line(format!("pub {}: {},", f.name.text, f.typ.as_rust()));
                    }
                });

                s.line("");
                s.line("impl Results {");
                write_downgrade(&mut s, "Results");
                s.line("}"); // impl Results
            }

            if let Some(body) = fun.body() {
                visit_ns_body(&mut s, body, depth + 1)?;
            }

            s.line("}");
            s.line("");
        }
        Ok(())
    };

    for fun in ns.local_funs() {
        write_fun(s, fun)?;
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
            let mut output = std::io::BufWriter::new(File::create(&mod_path)?);
            let mut s = Scope::new(&mut output);
            self.write_prelude(&mut s);

            for member in workspace.members.values() {
                s.line(format!("pub mod {};", member.name));
            }
        }

        Ok(())
    }
}

impl Generator {
    fn write_prelude<'a>(&self, s: &mut Scope<'a>) {
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
        let mut output = std::io::BufWriter::new(File::create(&output_path).unwrap());

        let mut s = Scope::new(&mut output);
        self.write_prelude(&mut s);

        let schema = member.schema.as_ref().expect("schema to be parsed");
        let ctx = Context::new(&schema.body);

        fn write_enum<'a, I>(s: &mut Scope, kind: &str, funs: I)
        where
            I: Iterator<Item = &'a Fun<'a>>,
        {
            let mut s = s.scope();
            for fun in funs {
                s.line(format!(
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
            let mut s = s.scope();

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
            write_enum(&mut s, "Params", ctx.funs(FunKind::Request));
            s.line("}"); // enum Params

            s.line("");
            s.line("#[derive(Serialize, Debug)]");
            s.line("#[serde(untagged)]");
            s.line("#[allow(non_camel_case_types, unused)]");
            s.line("pub enum Results {");
            write_enum(&mut s, "Results", ctx.funs(FunKind::Request));
            s.line("}"); // enum Results

            s.line("");
            s.line("#[derive(Serialize, Debug)]");
            s.line("#[serde(untagged)]");
            s.line("#[allow(non_camel_case_types, unused)]");
            s.line("pub enum NotificationParams {");
            write_enum(&mut s, "Params", ctx.funs(FunKind::Notification));
            s.line("}"); // enum NotificationParams

            s.line("");
            s.line("pub type Message = rpc::Message<Params, NotificationParams, Results>;");
            s.line("pub type RootClient = rpc::Client<Params, NotificationParams, Results>;");
            s.line("pub type Protocol = rpc::Protocol<Params, NotificationParams, Results>;");
            s.line("");
            s.line("pub fn protocol() -> Protocol {");
            s.in_scope(|s| {
                s.line("Protocol::new()");
            });
            s.line("}"); // fn protocol

            s.line("");
            s.line("pub struct Client {");
            s.in_scope(|s| {
                s.line("root: RootClient,");
            });
            s.line("}"); // struct Client

            s.line("");
            s.line("impl Client {");
            s.in_scope(|s| {
                for fun in ctx.funs(FunKind::Request) {
                    let params = fun.params();
                    let results = fun.results();

                    let params_def = if params.is_empty() {
                        "".into()
                    } else {
                        format!(", p: {}", params.qualified_type())
                    };

                    s.line(format!(
                        "pub async fn {}(&self{}) -> Result<{}, lavish_rpc::Error> {{",
                        fun.variant_name(),
                        params_def,
                        results.qualified_type()
                    ));
                    s.in_scope(|s| {
                        s.line("self.root.call(");
                        s.in_scope(|s| {
                            if params.is_empty() {
                                s.line(format!(
                                    "{}({}),",
                                    params.variant(),
                                    params.empty_literal()
                                ));
                            } else {
                                s.line(format!("{}(p),", params.variant()));
                            }
                            s.line(format!("{}::downgrade,", results.qualified_type()));
                        }); // h.call arguments
                        s.line(").await"); // h.call
                    });
                    s.line("}");
                    s.line("");
                }
            });
            s.line("}"); // impl Client

            for (strukt, side, kind) in &[
                ("Params", "Params", FunKind::Request),
                ("Results", "Results", FunKind::Request),
                ("Params", "NotificationParams", FunKind::Notification),
            ] {
                s.line("");
                s.line(format!("impl rpc::Atom for {} {{", side));
                s.in_scope(|s| {
                    s.line("fn method(&self) -> &'static str {");
                    s.in_scope(|s| {
                        s.line("match self {");
                        s.in_scope(|s| {
                            let mut count = 0;
                            for fun in ctx.funs(*kind) {
                                count += 1;
                                s.line(format!(
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
                    s.in_scope(|s| {
                        s.line("method: &str,");
                        s.line("de: &mut erased_serde::Deserializer,");
                    });
                    s.line(") -> erased_serde::Result<Self> {");
                    s.in_scope(|s| {
                        s.line("use erased_serde::deserialize as deser;");
                        s.line("use serde::de::Error;");
                        s.line("");
                        s.line("match method {");
                        s.in_scope(|s| {
                            for fun in ctx.funs(*kind) {
                                s.line(format!("{:?} =>", fun.rpc_name(),));
                                {
                                    let mut s = s.scope();
                                    s.line(format!(
                                        "Ok({}::{}(deser::<{}::{}>(de)?)),",
                                        side,
                                        fun.variant_name(),
                                        fun.qualified_name(),
                                        strukt,
                                    ));
                                }
                            }
                            s.line("_ => Err(erased_serde::Error::custom(format!(");
                            s.in_scope(|s| {
                                s.line(format!("{:?},", "unknown method: {}"));
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
            s.in_scope(|s| {
                s.line("pub state: Arc<T>,");
                s.line("pub client: Client,");
                s.line("pub params: PP,");
            });
            s.line("}"); // struct Call

            s.line("");
            s.line("pub type SlotFuture = ");
            s.in_scope(|s| {
                s.line("Future<Output = Result<Results, rpc::Error>> + Send + 'static;");
            });

            s.line("");
            s.line("pub type SlotReturn = Pin<Box<SlotFuture>>;");

            s.line("");
            s.line("pub type SlotFn<T> = ");
            s.in_scope(|s| {
                s.line("Fn(Arc<T>, Client, Params) -> SlotReturn + 'static + Send + Sync;");
            });

            s.line("");
            s.line("pub type Slot<T> = Option<Box<SlotFn<T>>>;");

            s.line("");
            s.line("pub struct Handler<T> {");
            s.in_scope(|s| {
                s.line("state: Arc<T>,");
                for fun in ctx.funs(FunKind::Request) {
                    s.line(format!("{}: Slot<T>,", fun.variant_name()));
                }
            });
            s.line("}"); // struct Handler

            s.line("");
            s.line("impl<T> Handler<T> {");
            s.in_scope(|s| {
                s.line("pub fn new(state: Arc<T>) -> Self {");
                s.in_scope(|s| {
                    s.line("Self {");
                    s.in_scope(|s| {
                        s.line("state,");
                        for fun in ctx.funs(FunKind::Request) {
                            s.line(format!("{}: None,", fun.variant_name()));
                        }
                    });
                    s.line("}");
                });
                s.line("}");

                for fun in ctx.funs(FunKind::Request) {
                    let params = fun.params();
                    let results = fun.results();

                    s.line("");
                    s.line(format!(
                        "pub fn on_{}<F, FT> (&mut self, f: F)",
                        fun.variant_name()
                    ));
                    s.line("where");
                    s.in_scope(|s| {
                        s.line(format!(
                            "F: Fn(Call<T, {}>) -> FT + Sync + Send + 'static,",
                            params.qualified_type()
                        ));
                        s.line(format!(
                            "FT: Future<Output = Result<{}, lavish_rpc::Error>> + Send + 'static,",
                            results.short_type()
                        ));
                    });
                    s.line("{");
                    s.in_scope(|s| {
                        s.line(format!(
                            "self.{} = Some(Box::new(move |state, client, params| {{",
                            fun.variant_name(),
                        ));
                        s.in_scope(|s| {
                            s.line("Box::pin(");
                            s.in_scope(|s| {
                                s.line("f(Call {");
                                s.in_scope(|s| {
                                    s.line("state, client,");
                                    s.line(format!(
                                        "params: {}::downgrade(params).unwrap(),",
                                        params.qualified_type()
                                    ));
                                });
                                if results.is_empty() {
                                    s.line(format!(
                                        "}}).map_ok(|_| {}({}))",
                                        results.variant(),
                                        results.empty_literal(),
                                    ));
                                } else {
                                    s.line(format!("}}).map_ok({})", results.variant()));
                                }
                            });
                            s.line(")");
                        });
                        s.line("}));");
                    });
                    s.line("}");
                }
                s.line("");
            });
            s.line("}"); // impl Handler

            s.line("");
            s.line("type HandlerRet = Pin<Box<dyn Future<Output = Result<Results, rpc::Error>> + Send + 'static>>;");
            s.line("");
            s.line("impl<T> rpc::Handler<Params, NotificationParams, Results, HandlerRet> for Handler<T>");
            s.line("where");
            s.in_scope(|s| {
                s.line("T: Send + Sync,");
            });
            s.line("{");
            s.in_scope(|s| {
            s.line("fn handle(&self, client: RootClient, params: Params) -> HandlerRet {");
            s.in_scope(|s| {
                s.line("let method = params.method();");
                s.line("let slot = match params {");
                s.in_scope(|s| {
                    for fun in ctx.funs(FunKind::Request) {
                        s.line(format!(
                            "Params::{}(_) => self.{}.as_ref(),",
                            fun.variant_name(),
                            fun.variant_name()
                        ));
                    }
                    s.line("_ => None,");
                });
                s.line("};");
                s.line("match slot {");
                s.in_scope(|s| {
                    s.line("Some(slot_fn) => {");
                    s.in_scope(|s| {
                        s.line("let res = slot_fn(self.state.clone(), Client { root: client }, params);");
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
            visit_ns_body(&mut s, &ctx.root, 0)?;

            s.line("");
            s.line("pub struct PeerBuilder<C>");
            s.line("where");
            s.in_scope(|s| {
                s.line("C: lavish_rpc::Conn,");
            });
            s.line("{");
            s.in_scope(|s| {
                s.line("conn: C,");
                s.line("pool: futures::executor::ThreadPool,");
            });
            s.line("}");

            s.line("");
            s.line("impl<C> PeerBuilder<C>");
            s.line("where");
            s.in_scope(|s| {
                s.line("C: lavish_rpc::Conn,");
            });
            s.line("{");
            s.in_scope(|s| {
                s.line("pub fn new(conn: C, pool: futures::executor::ThreadPool) -> Self {");
                s.in_scope(|s| {
                    s.line("Self { conn, pool }");
                });
                s.line("}");

                s.line("");
                s.line("pub fn with_noop_handler(self) -> Result<Client, lavish_rpc::Error> {");
                s.in_scope(|s| {
                    s.line("self.with_handler(|_| {})");
                });
                s.line("}");

                s.line("");
                s.line("pub fn with_handler<S>(self, setup: S) -> Result<Client, lavish_rpc::Error>");
                s.line("where");
                s.in_scope(|s| {
                    s.line("S: Fn(&mut Handler<()>),");
                });
                s.line("{");
                s.in_scope(|s| {
                    s.line("self.with_stateful_handler(std::sync::Arc::new(()), setup)");
                });
                s.line("}");

                s.line("");
                s.line(
                    "pub fn with_stateful_handler<T, S>(self, state: Arc<T>, setup: S) -> Result<Client, lavish_rpc::Error>",
                );
                s.line("where");
                s.in_scope(|s| {
                    s.line("S: Fn(&mut Handler<T>),");
                    s.line("T: Sync + Send + 'static,");
                });
                s.line("{");
                s.in_scope(|s| {
                    s.line("let mut handler = Handler::new(state);");
                    s.line("setup(&mut handler);");
                    s.line("let root = lavish_rpc::connect(protocol(), handler, self.conn, self.pool)?;");
                    s.line("Ok(Client { root })");
                });
                s.line("}");
            });
            s.line("}"); // impl PeerBuilder

            s.line("");
            s.line(
                "pub fn peer<C>(conn: C, pool: futures::executor::ThreadPool) -> PeerBuilder<C>",
            );
            s.line("where");
            s.in_scope(|s| {
                s.line("C: lavish_rpc::Conn,");
            });
            s.line("{");
            s.in_scope(|s| {
                s.line("PeerBuilder::new(conn, pool)");
                // FIXME: WIP
                // writeln!(s, "PeerBuilder::new(conn, pool)");
            });
            s.line("}"); // fn peer
        }

        s.line("}"); // mod __

        let end_instant = Instant::now();
        println!(
            "Generated {:?} in {:?}",
            output_path,
            end_instant.duration_since(start_instant)
        );

        Ok(())
    }
}
