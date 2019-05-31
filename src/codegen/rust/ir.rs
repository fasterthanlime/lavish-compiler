use heck::SnakeCase;
use indexmap::IndexMap;
use std::fmt::{self, Display, Write};

use crate::ast;
use crate::codegen::*;
use super::lang::*;

pub struct Namespace<'a> {
    name: &'a str,

    children: IndexMap<&'a str, Namespace<'a>>,
    pub funs: IndexMap<&'a str, Fun<'a>>,
    strus: IndexMap<&'a str, Stru<'a>>,
}

impl<'a> Namespace<'a> {
    pub fn new(prefix: &str, name: &'a str, decl: &'a ast::NamespaceBody) -> Self {
        let prefix = if name == "<root>" {
            "".into()
        } else {
            format!("{}{}.", prefix, name)
        };

        let mut children: IndexMap<&'a str, Namespace<'a>> = IndexMap::new();
        let mut funs: IndexMap<&'a str, Fun<'a>> = IndexMap::new();
        let mut strus: IndexMap<&'a str, Stru<'a>> = IndexMap::new();

        for decl in &decl.functions {
            let ff = Fun::new(&prefix, decl);
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

    pub fn funs(&self) -> Box<Iterator<Item = &'a Fun> + 'a> {
        Box::new(
            self.children
                .values()
                .map(Namespace::funs)
                .flatten()
                .chain(self.funs.values().map(Fun::funs).flatten()),
        )
    }

    pub fn local_funs(&'a self) -> impl Iterator<Item = &'a Fun> {
        self.funs.values()
    }

    pub fn name(&self) -> &'a str {
        self.name
    }

    pub fn children(&self) -> &IndexMap<&'a str, Namespace<'a>> {
        &self.children
    }

    pub fn strus(&self) -> &IndexMap<&'a str, Stru<'a>> {
        &self.strus
    }
}

pub enum FunStructKind {
    Params,
    Results,
}

pub struct FunStruct<'a> {
    pub fun: &'a Fun<'a>,
    pub kind: FunStructKind,
    pub fields: &'a Vec<ast::Field>,
}

impl<'a> FunStruct<'a> {
    pub fn kind(&self) -> &str {
        match self.kind {
            FunStructKind::Params => "Params",
            FunStructKind::Results => "Results",
        }
    }

    pub fn variant(&self) -> String {
        format!("{}::{}", self.kind(), self.fun.variant())
    }

    pub fn qualified_type(&self) -> String {
        format!("{}::{}", self.fun.qualified_name(), self.kind())
    }

    pub fn short_type(&self) -> String {
        if self.is_empty() {
            "()".into()
        } else {
            self.qualified_type()
        }
    }

    pub fn is_empty(&self) -> bool {
        self.fields.is_empty()
    }

    pub fn fields(&self) -> &Vec<ast::Field> {
        self.fields
    }

    pub fn empty_literal(&self) -> String {
        format!("{} {{}}", self.qualified_type())
    }
}

pub struct Atom<'a> {
    pub proto: &'a Protocol<'a>,
    pub name: &'a str,
    pub kind: ast::Kind,
    pub depth: usize,
}

impl<'a> Atom<'a> {
    fn funs(&self) -> impl Iterator<Item = &&Fun> {
        let kind = self.kind;
        self.proto.funs.iter().filter(move |f| f.kind() == kind)
    }
}

impl<'a> Atom<'a> {
    fn root(&self) -> String {
        "super::".repeat(self.depth)
    }
}

impl<'a> Atom<'a> {
    fn implement_method(&self, s: &mut Scope) {
        _fn("method")
            .self_param("&self")
            .returns("&'static str")
            .body(|s| {
                if self.funs().count() == 0 {
                    writeln!(s, "panic!(\"no variants for {}\")", self.name).unwrap();
                    return;
                }

                s.write("match self");
                s.in_block(|s| {
                    for fun in self.funs() {
                        writeln!(
                            s,
                            "{name}::{variant}(_) => {lit},",
                            name = &self.name,
                            variant = fun.variant(),
                            lit = quoted(fun.rpc_name())
                        )
                        .unwrap();
                    }
                });
            })
            .write_to(s);
    }

    fn implement_deserialize(&self, s: &mut Scope) {
        _fn("deserialize")
            .param("method: &str")
            .param("de: &mut lavish_rpc::erased_serde::Deserializer")
            .returns("lavish_rpc::erased_serde::Result<Self>")
            .body(|s| {
                s.line("use lavish_rpc::erased_serde::deserialize as __DS;");
                s.line("use lavish_rpc::serde::de::Error;");
                s.lf();

                s.write("match method");
                s.in_block(|s| {
                    for fun in self.funs() {
                        s.line(format!("{rpc_name} => ", rpc_name = quoted(fun.rpc_name())));
                        s.scope()
                            .write("Ok")
                            .in_list(Brackets::Round, |l| {
                                l.item(format!(
                                    "{name}::{variant}(__DS::<{root}{qfn}::{name}>(de)?)",
                                    root = self.root(),
                                    name = &self.name,
                                    variant = fun.variant(),
                                    qfn = fun.qualified_name(),
                                ));
                            })
                            .write(",")
                            .lf();
                    }
                    s.write("_ =>").lf();
                    s.scope().write("Err").in_parens(|s| {
                        s.write("lavish_rpc::erased_serde::Error::custom")
                            .in_parens(|s| {
                                s.write("format!").in_parens_list(|l| {
                                    l.item(quoted("unknown method: {}"));
                                    l.item("method")
                                });
                            });
                    });
                });
            })
            .write_to(s);
    }
}

impl<'a> Display for Atom<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        Scope::fmt(f, |s| {
            s.write(derive().debug().serialize());
            s.write(allow().non_camel_case().unused());
            s.write(serde_untagged());
            let mut e = _enum(self.name).kw_pub();
            for fun in self.funs() {
                e = e.variant(format!(
                    "{variant}({root}{fqn}::{name})",
                    variant = fun.variant(),
                    root = self.root(),
                    fqn = fun.qualified_name(),
                    name = &self.name
                ));
            }
            s.write(e);

            let mut i = _impl("lavish_rpc::Atom", self.name).body(|s| {
                self.implement_method(s);
                self.implement_deserialize(s);
            });
            s.write(i);
        })
    }
}

pub struct Client<'a> {
    pub handled: &'a [&'a Fun<'a>],
    pub called: &'a [&'a Fun<'a>],
    pub depth: usize,
    pub is_root: bool,
}

impl<'a> Client<'a> {
    fn root(&self) -> String {
        "super::".repeat(self.depth + 1)
    }

    fn protocol(&self) -> String {
        format!("{root}protocol", root = self.root())
    }

    fn define_client(&self, s: &mut Scope) {
        s.write("pub struct Client");
        s.in_block(|s| {
            for fun in self.called {
                writeln!(s, "{variant}: (),", variant = fun.variant()).unwrap();
            }
            s.line("// TODO");
        });
        s.lf();
    }

    fn define_call(&self, s: &mut Scope) {
        s.write("pub struct Call<T, PP>");
        s.in_block(|s| {
            s.line("pub state: std::sync::Arc<T>,");
            s.line("pub client: Client,");
            s.line("pub params: PP,");
        });
        s.lf();
    }

    fn define_slot(&self, s: &mut Scope) {
        s.write("pub type SlotFuture = Future")
            .in_list(Brackets::Angle, |l| {
                l.item(format!("Output = Result<{protocol}::Results, lavish_rpc::Error>", protocol = self.protocol()));
            })
            .write(" + Send + 'static;")
            .lf();

        s.write("pub type SlotReturn = std::pin::Pin<Box<SlotFuture>>;")
            .lf();

        writeln!(s, 
            "pub type SlotFn<T> = Fn(std::sync::Arc<T>, Client, {protocol}::Params) -> SlotReturn + 'static + Send + Sync;",
            protocol = self.protocol(),
        ).unwrap();

        s.write("pub type Slot<T> = Option<Box<SlotFn<T>>>;").lf();
    }

    fn define_handler(&self, s: &mut Scope) {
        s.write("pub struct Handler<T>");
        s.in_block(|s| {
            s.line("// TODO");
            s.line("state: std::sync::Arc<T>,");
            for fun in self.handled {
                writeln!(s, "on_{name}: Slot<T>,", name = fun.name()).unwrap();
            }
        });
        s.lf();
    }
}

impl<'a> Display for Client<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        Scope::fmt(f, |s| {
            s.line("use futures::prelude::*;");
            self.define_client(s);
            self.define_call(s);
            self.define_slot(s);
            self.define_handler(s);
        })
    }
}

fn quoted<D>(d: D) -> String
where
    D: fmt::Debug,
{
    format!("{:?}", d)
}

pub struct _Enum {
    kw_pub: bool,
    name: String,
    annotations: Vec<String>,
    variants: Vec<String>,
}

impl _Enum {
    pub fn kw_pub(mut self) -> Self {
        self.kw_pub = true;
        self
    }

    pub fn annotation<D>(mut self, d: D) -> Self
    where
        D: Display,
    {
        self.annotations.push(format!("{}", d));
        self
    }

    pub fn variant<D>(mut self, d: D) -> Self
    where
        D: Display,
    {
        self.variants.push(format!("{}", d));
        self
    }
}

pub fn _enum<S>(name: S) -> _Enum
where
    S: Into<String>,
{
    _Enum {
        name: name.into(),
        kw_pub: false,
        annotations: Vec::new(),
        variants: Vec::new(),
    }
}

impl Display for _Enum {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        Scope::fmt(f, |s| {
            for annotation in &self.annotations {
                s.write(annotation);
            }
            if self.kw_pub {
                s.write("pub ");
            }
            s.write("enum ").write(&self.name);
            if self.variants.is_empty() {
                s.write(" {}").lf();
            } else {
                s.in_block(|s| {
                    for variant in &self.variants {
                        s.write(variant).write(",").lf();
                    }
                });
            }
        })
    }
}

pub struct Protocol<'a> {
    pub funs: &'a [&'a Fun<'a>],
    pub depth: usize,
}

impl<'a> Display for Protocol<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        Scope::fmt(f, |s| {
            s.write("pub mod protocol");
            s.in_block(|s| {
                let depth = self.depth + 1;
                for a in &[
                    Atom {
                        proto: &self,
                        kind: ast::Kind::Request,
                        name: "Params",
                        depth,
                    },
                    Atom {
                        proto: &self,
                        kind: ast::Kind::Request,
                        name: "Results",
                        depth,
                    },
                    Atom {
                        proto: &self,
                        kind: ast::Kind::Notification,
                        name: "NotificationParams",
                        depth,
                    },
                ] {
                    s.line(a);
                }
            });
        })
    }
}

pub struct Fun<'a> {
    pub decl: &'a ast::FunctionDecl,
    tokens: Vec<String>,

    body: Option<Namespace<'a>>,
}

impl<'a> Fun<'a> {
    pub fn new(prefix: &str, decl: &'a ast::FunctionDecl) -> Self {
        let name: &str = &decl.name.text;
        let full_name = format!("{}{}", prefix, name);
        Self {
            decl,
            tokens: full_name.split('.').map(|x| x.into()).collect(),
            body: decl.body.as_ref().map(|b| Namespace::new(prefix, name, b)),
        }
    }

    pub fn rpc_name(&self) -> String {
        self.tokens.join(".")
    }

    pub fn variant(&self) -> String {
        self.rpc_name().replace(".", "__").to_lowercase()
    }

    pub fn params(&'a self) -> FunStruct<'a> {
        FunStruct {
            fun: self,
            fields: &self.decl.params,
            kind: FunStructKind::Params,
        }
    }

    pub fn results(&'a self) -> FunStruct<'a> {
        FunStruct {
            fun: self,
            fields: &self.decl.results,
            kind: FunStructKind::Results,
        }
    }

    pub fn qualified_name(&self) -> String {
        self.tokens.join("::")
    }

    pub fn mod_name(&self) -> String {
        self.decl.name.text.to_snake_case()
    }

    pub fn comment(&self) -> &Option<ast::Comment> {
        &self.decl.comment
    }

    pub fn funs(&self) -> Box<Iterator<Item = &'a Fun> + 'a> {
        let iter = std::iter::once(self);
        if let Some(body) = self.body.as_ref() {
            Box::new(iter.chain(body.funs()))
        } else {
            Box::new(iter)
        }
    }

    pub fn body(&self) -> Option<&Namespace<'a>> {
        self.body.as_ref()
    }

    pub fn name(&self) -> &str {
        self.decl.name.text.as_ref()
    }

    pub fn side(&self) -> ast::Side {
        self.decl.side
    }

    pub fn kind(&self) -> ast::Kind {
        self.decl.kind
    }
}

pub struct Stru<'a> {
    decl: &'a ast::StructDecl,
    #[allow(unused)]
    full_name: String,
}

impl<'a> Stru<'a> {
    pub fn new(decl: &'a ast::StructDecl, full_name: String) -> Self {
        Self { decl, full_name }
    }

    pub fn comment(&self) -> &Option<ast::Comment> {
        &self.decl.comment
    }

    pub fn name(&self) -> &str {
        &self.decl.name.text
    }

    pub fn fields(&self) -> &Vec<ast::Field> {
        &self.decl.fields
    }
}
