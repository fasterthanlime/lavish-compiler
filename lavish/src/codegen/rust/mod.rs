use crate::ast;
use crate::codegen::Result;
use std::fmt::Write;
use std::fs::File;
use std::time::Instant;

mod ir;
use ir::*;

mod output;
use output::*;

struct Context<'a> {
    root: Namespace<'a>,
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
    writeln!(s, "pub mod {} {{", ns.name())?;
    {
        let mut s = s.scope();
        visit_ns_body(&mut s, ns, depth)?;
    }
    writeln!(s, "}}")?; // pub mod
    writeln!(s)?;
    Ok(())
}

fn visit_ns_body<'a>(s: &mut Scope<'a>, ns: &'a Namespace<'a>, depth: usize) -> Result {
    for (_, ns) in ns.children() {
        visit_ns(s, ns, depth + 1)?;
    }

    writeln!(s, "use lavish_rpc::serde_derive::*;")?;
    writeln!(s)?;

    for (_, st) in ns.strus() {
        s.comment(&st.comment());
        s.def_struct(st.name(), |s| {
            for f in st.fields() {
                s.comment(&f.comment);
                writeln!(s, "pub {}: {},", f.name.text, f.typ.as_rust())?;
            }
            Ok(())
        })?;
        writeln!(s)?;
    }

    let write_fun = |s: &mut Scope, fun: &Fun<'a>| -> Result {
        s.comment(fun.comment());
        writeln!(s, "pub mod {} {{", fun.mod_name())?;

        {
            let mut s = s.scope();
            writeln!(s, "use futures::prelude::*;")?;
            writeln!(s, "use lavish_rpc::serde_derive::*;")?;
            let super_ref = "super::".repeat(depth + 2);
            writeln!(s, "use {}__;", super_ref)?;
            writeln!(s)?;

            let write_downgrade = |s: &mut Scope, side: &str| -> Result {
                s.in_scope(|s| {
                    writeln!(s, "pub fn downgrade(p: __::{}) -> Option<Self> {{", side,)?;
                    s.in_scope(|s| {
                        writeln!(s, "match p {{")?;
                        s.in_scope(|s| {
                            s.line(format!(
                                "__::{}::{}(p) => Some(p),",
                                side,
                                fun.variant()
                            ));
                            writeln!(s, "_ => None,")?;
                            Ok(())
                        })?;
                        writeln!(s, "}}")?; // match p
                        Ok(())
                    })?;
                    writeln!(s, "}}")?; // fn downgrade
                    Ok(())
                })?;
                Ok(())
            };

            s.def_struct("Params", |s| {
                for f in fun.params().fields() {
                    writeln!(s, "pub {}: {},", f.name.text, f.typ.as_rust())?;
                }
                Ok(())
            })?;

            writeln!(s)?;
            writeln!(s, "impl Params {{")?;
            write_downgrade(
                &mut s,
                if fun.is_notification() {
                    "NotificationParams"
                } else {
                    "Params"
                },
            )?;
            writeln!(s, "}}")?; // impl Params

            if !fun.is_notification() {
                writeln!(s)?;
                s.def_struct("Results", |s| {
                    for f in fun.results().fields() {
                        writeln!(s, "pub {}: {},", f.name.text, f.typ.as_rust())?;
                    }
                    Ok(())
                })?;

                writeln!(s)?;
                writeln!(s, "impl Results {{")?;
                write_downgrade(&mut s, "Results")?;
                writeln!(s, "}}")?; // impl Results
            }

            if let Some(body) = fun.body() {
                visit_ns_body(&mut s, body, depth + 1)?;
            }

            writeln!(s, "}}")?;
            writeln!(s)?;
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
            let mut output = Scope::writer(File::create(&mod_path)?);
            let mut s = Scope::new(&mut output);
            self.write_prelude(&mut s)?;

            for member in workspace.members.values() {
                writeln!(s, "pub mod {};", member.name)?;
            }
        }

        Ok(())
    }
}

impl Generator {
    fn write_prelude<'a>(&self, s: &mut Scope<'a>) -> Result {
        writeln!(s, "// This file is generated by lavish: DO NOT EDIT")?;
        writeln!(s, "// https://github.com/fasterthanlime/lavish")?;
        writeln!(s)?;
        writeln!(s, "#![cfg_attr(rustfmt, rustfmt_skip)]")?;
        writeln!(s, "#![allow(clippy::all)]")?;
        writeln!(s, "#![allow(unknown_lints)]")?;
        writeln!(s, "#![allow(unused)]")?;
        writeln!(s)?;
        Ok(())
    }

    fn emit(&self, workspace: &ast::Workspace, member: &ast::WorkspaceMember) -> Result {
        let start_instant = Instant::now();

        let output_path = workspace.dir.join(&member.name).join("mod.rs");
        std::fs::create_dir_all(output_path.parent().unwrap())?;
        let mut output = Scope::writer(File::create(&output_path)?);
        let mut s = Scope::new(&mut output);
        self.write_prelude(&mut s)?;

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
                    fun.variant(),
                    fun.qualified_name(),
                    kind,
                ));
            }
        };

        {
            writeln!(s, "pub use __::*;")?;
            writeln!(s)?;

            writeln!(s)?;
            writeln!(s, "//============= FIXME: experimental (start)")?;
            {
                let funs = ctx.all_funs().collect::<Vec<_>>();
                writeln!(s, "{}", Protocol { funs: &funs[..], depth: 0 })?;
            }
            writeln!(s, "//============= FIXME: experimental (end)")?;
            writeln!(s)?;

            writeln!(s, "mod __ {{")?;
            let mut s = s.scope();

            writeln!(s, "// Notes: as of 2019-05-21, futures-preview is required")?;
            writeln!(s, "use futures::prelude::*;")?;
            writeln!(s, "use std::pin::Pin;")?;
            writeln!(s, "use std::sync::Arc;")?;
            writeln!(s)?;
            writeln!(s, "use lavish_rpc as rpc;")?;
            writeln!(s, "use rpc::{{Atom, erased_serde, serde_derive::*}};")?;

            writeln!(s)?;
            writeln!(s, "#[derive(Serialize, Debug)]")?;
            writeln!(s, "#[serde(untagged)]")?;
            writeln!(s, "#[allow(non_camel_case_types, unused)]")?;
            writeln!(s, "pub enum Params {{")?;
            write_enum(&mut s, "Params", ctx.funs(FunKind::Request));
            writeln!(s, "}}")?; // enum Params

            writeln!(s)?;
            writeln!(s, "#[derive(Serialize, Debug)]")?;
            writeln!(s, "#[serde(untagged)]")?;
            writeln!(s, "#[allow(non_camel_case_types, unused)]")?;
            writeln!(s, "pub enum Results {{")?;
            write_enum(&mut s, "Results", ctx.funs(FunKind::Request));
            writeln!(s, "}}")?; // enum Results

            writeln!(s)?;
            writeln!(s, "#[derive(Serialize, Debug)]")?;
            writeln!(s, "#[serde(untagged)]")?;
            writeln!(s, "#[allow(non_camel_case_types, unused)]")?;
            writeln!(s, "pub enum NotificationParams {{")?;
            write_enum(&mut s, "Params", ctx.funs(FunKind::Notification));
            writeln!(s, "}}")?; // enum NotificationParams

            writeln!(s)?;
            writeln!(
                s,
                "pub type Message = rpc::Message<Params, NotificationParams, Results>;"
            )?;
            writeln!(
                s,
                "pub type RootClient = rpc::Client<Params, NotificationParams, Results>;"
            )?;
            writeln!(
                s,
                "pub type Protocol = rpc::Protocol<Params, NotificationParams, Results>;"
            )?;
            writeln!(s)?;
            writeln!(s, "pub fn protocol() -> Protocol {{")?;
            s.in_scope(|s| {
                writeln!(s, "Protocol::new()")?;
                Ok(())
            })?;
            writeln!(s, "}}")?; // fn protocol

            writeln!(s)?;
            writeln!(s, "pub struct Client {{")?;
            s.in_scope(|s| {
                writeln!(s, "root: RootClient,")?;
                Ok(())
            })?;
            writeln!(s, "}}")?; // struct Client

            writeln!(s)?;
            writeln!(s, "impl Client {{")?;
            s.in_scope(|s| {
                for fun in ctx.funs(FunKind::Request) {
                    let params = fun.params();
                    let results = fun.results();

                    let params_def = if params.is_empty() {
                        "".into()
                    } else {
                        format!(", p: {}", params.qualified_type())
                    };

                    writeln!(
                        s,
                        "pub async fn {}(&self{}) -> Result<{}, lavish_rpc::Error> {{",
                        fun.variant(),
                        params_def,
                        results.qualified_type()
                    )?;
                    s.in_scope(|s| {
                        writeln!(s, "self.root.call(")?;
                        s.in_scope(|s| {
                            if params.is_empty() {
                                s.line(format!(
                                    "{}({}),",
                                    params.variant(),
                                    params.empty_literal()
                                ));
                            } else {
                                writeln!(s, "{}(p),", params.variant())?;
                            }
                            writeln!(s, "{}::downgrade,", results.qualified_type())?;
                            Ok(())
                        })?; // h.call arguments
                        writeln!(s, ").await")?; // h.call
                        Ok(())
                    })?;
                    writeln!(s, "}}")?;
                    writeln!(s)?;
                }
                Ok(())
            })?;
            writeln!(s, "}}")?; // impl Client

            for (strukt, side, kind) in &[
                ("Params", "Params", FunKind::Request),
                ("Results", "Results", FunKind::Request),
                ("Params", "NotificationParams", FunKind::Notification),
            ] {
                writeln!(s)?;
                writeln!(s, "impl rpc::Atom for {} {{", side)?;
                s.in_scope(|s| {
                    writeln!(s, "fn method(&self) -> &'static str {{")?;
                    s.in_scope(|s| {
                        writeln!(s, "match self {{")?;
                        s.in_scope(|s| {
                            let mut count = 0;
                            for fun in ctx.funs(*kind) {
                                count += 1;
                                writeln!(s, 
                                    "{}::{}(_) => {:?},",
                                    side,
                                    fun.variant(),
                                    fun.rpc_name()
                                )?;
                            }
                            if count == 0 {
                                writeln!(s, "_ => unimplemented!()")?;
                            }
                            Ok(())
                        })?;
                        writeln!(s, "}}")?;
                        Ok(())
                    })?;
                    writeln!(s, "}}")?; // fn method

                    writeln!(s)?;
                    writeln!(s, "fn deserialize(")?;
                    s.in_scope(|s| {
                        writeln!(s, "method: &str,")?;
                        writeln!(s, "de: &mut erased_serde::Deserializer,")?;
                        Ok(())
                    })?;
                    writeln!(s, ") -> erased_serde::Result<Self> {{")?;
                    s.in_scope(|s| {
                        writeln!(s, "use erased_serde::deserialize as deser;")?;
                        writeln!(s, "use serde::de::Error;")?;
                        writeln!(s)?;
                        writeln!(s, "match method {{")?;
                        s.in_scope(|s| {
                            for fun in ctx.funs(*kind) {
                                writeln!(s, "{:?} =>", fun.rpc_name(),)?;
                                {
                                    let mut s = s.scope();
                                    s.line(format!(
                                        "Ok({}::{}(deser::<{}::{}>(de)?)),",
                                        side,
                                        fun.variant(),
                                        fun.qualified_name(),
                                        strukt,
                                    ));
                                }
                            }
                            writeln!(s, "_ => Err(erased_serde::Error::custom(format!(")?;
                            s.in_scope(|s| {
                                writeln!(s, "{:?},", "unknown method: {}")?;
                                writeln!(s, "method,")?;
                                Ok(())
                            })?;
                            writeln!(s, "))),")?;
                            Ok(())
                        })?;
                        writeln!(s, "}}")?;
                        Ok(())
                    })?;
                    writeln!(s, "}}")?; // fn deserialize
                    Ok(())
                })?;
                writeln!(s, "}}")?; // impl Atom for side
            } // impl rpc::Atom for P, NP, R

            writeln!(s)?;
            writeln!(s, "pub struct Call<T, PP> {{")?;
            s.in_scope(|s| {
                writeln!(s, "pub state: Arc<T>,")?;
                writeln!(s, "pub client: Client,")?;
                writeln!(s, "pub params: PP,")?;
                Ok(())
            })?;
            writeln!(s, "}}")?; // struct Call

            writeln!(s)?;
            writeln!(s, "pub type SlotFuture =")?;
            s.in_scope(|s| {
                writeln!(
                    s,
                    "Future<Output = Result<Results, rpc::Error>> + Send + 'static;"
                )?;
                Ok(())
            })?;

            writeln!(s)?;
            writeln!(s, "pub type SlotReturn = Pin<Box<SlotFuture>>;")?;

            writeln!(s)?;
            writeln!(s, "pub type SlotFn<T> =")?;
            s.in_scope(|s| {
                writeln!(
                    s,
                    "Fn(Arc<T>, Client, Params) -> SlotReturn + 'static + Send + Sync;"
                )?;
                Ok(())
            })?;

            writeln!(s)?;
            writeln!(s, "pub type Slot<T> = Option<Box<SlotFn<T>>>;")?;

            writeln!(s)?;
            writeln!(s, "pub struct Handler<T> {{")?;
            s.in_scope(|s| {
                writeln!(s, "state: Arc<T>,")?;
                for fun in ctx.funs(FunKind::Request) {
                    writeln!(s, "{}: Slot<T>,", fun.variant())?;
                }
                Ok(())
            })?;
            writeln!(s, "}}")?; // struct Handler

            writeln!(s)?;
            writeln!(s, "impl<T> Handler<T> {{")?;
            s.in_scope(|s| {
                writeln!(s, "pub fn new(state: Arc<T>) -> Self {{")?;
                s.in_scope(|s| {
                    writeln!(s, "Self {{")?;
                    s.in_scope(|s| {
                        writeln!(s, "state,")?;
                        for fun in ctx.funs(FunKind::Request) {
                            writeln!(s, "{}: None,", fun.variant())?;
                        }
                        Ok(())
                    })?;
                    writeln!(s, "}}")?;
                    Ok(())
                })?;
                writeln!(s, "}}")?;

                for fun in ctx.funs(FunKind::Request) {
                    let params = fun.params();
                    let results = fun.results();

                    writeln!(s)?;
                    s.line(format!(
                        "pub fn on_{}<F, FT> (&mut self, f: F)",
                        fun.variant()
                    ));
                    writeln!(s, "where")?;
                    s.in_scope(|s| {
                        s.line(format!(
                            "F: Fn(Call<T, {}>) -> FT + Sync + Send + 'static,",
                            params.qualified_type()
                        ));
                        s.line(format!(
                            "FT: Future<Output = Result<{}, lavish_rpc::Error>> + Send + 'static,",
                            results.short_type()
                        ));
                        Ok(())
                    })?;
                    writeln!(s, "{{")?;
                    s.in_scope(|s| {
                        s.line(format!(
                            "self.{} = Some(Box::new(move |state, client, params| {{",
                            fun.variant(),
                        ));
                        s.in_scope(|s| {
                            writeln!(s, "Box::pin(")?;
                            s.in_scope(|s| {
                                writeln!(s, "f(Call {{")?;
                                s.in_scope(|s| {
                                    writeln!(s, "state, client,")?;
                                    s.line(format!(
                                        "params: {}::downgrade(params).unwrap(),",
                                        params.qualified_type()
                                    ));
                                    Ok(())
                                })?;
                                if results.is_empty() {
                                    writeln!(
                                        s,
                                        "}}).map_ok(|_| {}({}))",
                                        results.variant(),
                                        results.empty_literal(),
                                    )?;
                                } else {
                                    writeln!(s, "}}).map_ok({})", results.variant())?;
                                }
                                Ok(())
                            })?;
                            writeln!(s, ")")?;
                            Ok(())
                        })?;
                        writeln!(s, "}}));")?;
                        Ok(())
                    })?;
                    writeln!(s, "}}")?;
                }
                writeln!(s)?;
                Ok(())
            })?;
            writeln!(s, "}}")?; // impl Handler

            writeln!(s)?;
            writeln!(s, "type HandlerRet = Pin<Box<dyn Future<Output = Result<Results, rpc::Error>> + Send + 'static>>;")?;
            writeln!(s)?;
            writeln!(s, "impl<T> rpc::Handler<Params, NotificationParams, Results, HandlerRet> for Handler<T>")?;
            writeln!(s, "where")?;
            s.in_scope(|s| {
                writeln!(s, "T: Send + Sync,")?;
                Ok(())
            })?;
            writeln!(s, "{{")?;
            s.in_scope(|s| {
            writeln!(s, "fn handle(&self, client: RootClient, params: Params) -> HandlerRet {{")?;
            s.in_scope(|s| {
                writeln!(s, "let method = params.method();")?;
                writeln!(s, "let slot = match params {{")?;
                s.in_scope(|s| {
                    for fun in ctx.funs(FunKind::Request) {
                        s.line(format!(
                            "Params::{}(_) => self.{}.as_ref(),",
                            fun.variant(),
                            fun.variant()
                        ));
                    }
                    writeln!(s, "_ => None,")?;
                    Ok(())
                })?;
                writeln!(s, "}};")?;
                writeln!(s, "match slot {{")?;
                s.in_scope(|s| {
                    writeln!(s, "Some(slot_fn) => {{")?;
                    s.in_scope(|s| {
                        writeln!(s, "let res = slot_fn(self.state.clone(), Client {{ root: client }}, params);")?;
                        writeln!(s, "Box::pin(async move {{ Ok(res.await?) }})")?;
                        Ok(())
                    })?;
                    writeln!(s, "}}")?; // Some(slot_fn)?
                    writeln!(s, "None => Box::pin(async move {{ Err(rpc::Error::MethodUnimplemented(method)) }}),")?;
                    Ok(())
                })?;
                writeln!(s, "}}")?; // match slot
                Ok(())
            })?;
            writeln!(s, "}}")?;
            Ok(())
        })?;
            writeln!(s, "}}")?; // impl rpc::Handler for Handler

            writeln!(s)?;
            visit_ns_body(&mut s, &ctx.root, 0)?;

            writeln!(s)?;
            writeln!(s, "pub struct PeerBuilder<C>")?;
            writeln!(s, "where")?;
            s.in_scope(|s| {
                writeln!(s, "C: lavish_rpc::Conn,")?;
                Ok(())
            })?;
            writeln!(s, "{{")?;
            s.in_scope(|s| {
                writeln!(s, "conn: C,")?;
                writeln!(s, "pool: futures::executor::ThreadPool,")?;
                Ok(())
            })?;
            writeln!(s, "}}")?;

            writeln!(s)?;
            writeln!(s, "impl<C> PeerBuilder<C>")?;
            writeln!(s, "where")?;
            s.in_scope(|s| {
                writeln!(s, "C: lavish_rpc::Conn,")?;
                Ok(())
            })?;
            writeln!(s, "{{")?;
            s.in_scope(|s| {
                writeln!(s, "pub fn new(conn: C, pool: futures::executor::ThreadPool) -> Self {{")?;
                s.in_scope(|s| {
                    writeln!(s, "Self {{ conn, pool }}")?;
                    Ok(())
                })?;
                writeln!(s, "}}")?;

                writeln!(s)?;
                writeln!(s, "pub fn with_noop_handler(self) -> Result<Client, lavish_rpc::Error> {{")?;
                s.in_scope(|s| {
                    writeln!(s, "self.with_handler(|_| {{}})")?;
                    Ok(())
                })?;
                writeln!(s, "}}")?;

                writeln!(s)?;
                writeln!(s, "pub fn with_handler<S>(self, setup: S) -> Result<Client, lavish_rpc::Error>")?;
                writeln!(s, "where")?;
                s.in_scope(|s| {
                    writeln!(s, "S: Fn(&mut Handler<()>),")?;
                    Ok(())
                })?;
                writeln!(s, "{{")?;
                s.in_scope(|s| {
                    writeln!(s, "self.with_stateful_handler(std::sync::Arc::new(()), setup)")?;
                    Ok(())
                })?;
                writeln!(s, "}}")?;

                writeln!(s)?;
                writeln!(s, 
                    "pub fn with_stateful_handler<T, S>(self, state: Arc<T>, setup: S) -> Result<Client, lavish_rpc::Error>",
                )?;
                writeln!(s, "where")?;
                s.in_scope(|s| {
                    writeln!(s, "S: Fn(&mut Handler<T>),")?;
                    writeln!(s, "T: Sync + Send + 'static,")?;
                    Ok(())
                })?;
                writeln!(s, "{{")?;
                s.in_scope(|s| {
                    writeln!(s, "let mut handler = Handler::new(state);")?;
                    writeln!(s, "setup(&mut handler);")?;
                    writeln!(s, "let root = lavish_rpc::connect(protocol(), handler, self.conn, self.pool)?;")?;
                    writeln!(s, "Ok(Client {{ root }})")?;
                    Ok(())
                })?;
                writeln!(s, "}}")?;
                Ok(())
            })?;
            writeln!(s, "}}")?; // impl PeerBuilder

            writeln!(s)?;
            s.line(
                "pub fn peer<C>(conn: C, pool: futures::executor::ThreadPool) -> PeerBuilder<C>",
            );
            writeln!(s, "where")?;
            s.in_scope(|s| {
                writeln!(s, "C: lavish_rpc::Conn,")?;
                Ok(())
            })?;
            writeln!(s, "{{")?;
            s.in_scope(|s| {
                writeln!(s, "PeerBuilder::new(conn, pool)")?;
                Ok(())
            })?;
            writeln!(s, "}}")?; // fn peer
        }
        writeln!(s, "}}")?; // mod __

        let end_instant = Instant::now();
        println!(
            "Generated {:?} in {:?}",
            output_path,
            end_instant.duration_since(start_instant)
        );

        Ok(())
    }
}
