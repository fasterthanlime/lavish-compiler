use heck::{CamelCase, SnakeCase};
use indexmap::IndexMap;

use crate::ast;

mod client;
mod common;
mod lang;
mod protocol;
mod symbols;

pub use client::*;
pub use common::*;
pub use lang::*;
pub use protocol::*;
pub use symbols::*;

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

pub trait Frame {
    fn name(&self) -> String;
    fn kind<'a>(&'a self) -> FrameKind<'a>;
}

pub enum FrameKind<'a> {
    Namespace(&'a ast::NamespaceDecl),
    Function(&'a ast::FunctionDecl),
}

impl Frame for ast::FunctionDecl {
    fn name(&self) -> String {
        self.name.text.clone()
    }

    fn kind<'a>(&'a self) -> FrameKind<'a> {
        FrameKind::Function(self)
    }
}

impl Frame for ast::NamespaceDecl {
    fn name(&self) -> String {
        self.name.text.clone()
    }

    fn kind<'a>(&'a self) -> FrameKind<'a> {
        FrameKind::Namespace(self)
    }
}

#[derive(Clone)]
pub struct Stack<'a> {
    frames: Vec<&'a Frame>,
}

impl<'a> Stack<'a> {
    pub fn new() -> Self {
        Self { frames: Vec::new() }
    }

    pub fn push(&self, frame: &'a Frame) -> Self {
        let mut frames = self.frames.clone();
        frames.push(frame);
        Self { frames }
    }

    pub fn anchor<T>(&self, inner: T) -> Anchored<T> {
        Anchored {
            stack: self.clone(),
            inner,
        }
    }

    pub fn names(&self) -> Vec<String> {
        self.frames.iter().map(|x| x.name()).collect()
    }

    pub fn trace(&self) -> String {
        self.names().join("::")
    }

    pub fn root(&self) -> String {
        "super::".repeat(self.frames.len() + 1)
    }

    pub fn protocol(&self) -> String {
        format!("{}protocol", self.root())
    }

    pub fn schema(&self) -> String {
        format!("{}schema", self.root())
    }
}

#[derive(Clone)]
pub struct Anchored<'a, T> {
    inner: T,
    stack: Stack<'a>,
}

impl<'a, T> std::ops::Deref for Anchored<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<'a, T> Anchored<'a, T> {
    pub fn inner(&self) -> &T {
        &self.inner
    }

    pub fn stack(&self) -> &Stack {
        &self.stack
    }
}

impl<'a> Anchored<'a, &ast::NamespaceBody> {
    pub fn walk_local_funs(&self, cb: &mut FnMut(Anchored<&ast::FunctionDecl>)) {
        for f in &self.functions {
            cb(self.stack.anchor(f));
        }
    }

    pub fn walk_local_namespaces(&self, cb: &mut FnMut(Anchored<&ast::NamespaceBody>)) {
        for ns in &self.namespaces {
            cb(self.stack.push(ns).anchor(&ns.body));
        }
    }

    pub fn walk_all_funs(&self, cb: &mut FnMut(Anchored<&ast::FunctionDecl>)) {
        self.walk_local_funs(&mut |f| {
            f.walk_all_funs(cb);
            cb(f);
        });
        self.walk_local_namespaces(&mut |ns| ns.walk_all_funs(cb));
    }

    pub fn walk_client_funs(&self, cb: &mut FnMut(Anchored<&ast::FunctionDecl>)) {
        self.walk_local_funs(cb);
        self.walk_local_namespaces(&mut |ns| ns.walk_client_funs(cb));
    }
}

impl<'a> Anchored<'a, &ast::FunctionDecl> {
    pub fn walk_all_funs(&self, cb: &mut FnMut(Anchored<&ast::FunctionDecl>)) {
        if let Some(body) = self.body.as_ref() {
            self.stack.push(self.inner).anchor(body).walk_all_funs(cb);
        }
    }

    fn names(&self) -> Vec<String> {
        let mut names = self.stack.names();
        names.push(self.name().into());
        names
    }

    pub fn variant(&self) -> String {
        self.names()
            .iter()
            .map(|x| x.to_camel_case())
            .collect::<Vec<_>>()
            .join("_")
    }

    pub fn module(&self) -> String {
        self.names().join("::")
    }

    pub fn method(&self) -> String {
        self.names().join(".")
    }

    pub fn qualified_name(&self) -> String {
        self.names().join("__")
    }

    pub fn name(&self) -> &str {
        self.inner.name.text.as_ref()
    }
}
