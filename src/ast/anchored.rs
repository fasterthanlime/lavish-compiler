use crate::ast::nodes::*;
use heck::CamelCase;

pub trait Frame {
    fn name(&self) -> String;
    fn kind<'a>(&'a self) -> FrameKind<'a>;
}

pub enum FrameKind<'a> {
    Namespace(&'a NamespaceDecl),
    Function(&'a FunctionDecl),
}

impl Frame for FunctionDecl {
    fn name(&self) -> String {
        self.name.text.clone()
    }

    fn kind<'a>(&'a self) -> FrameKind<'a> {
        FrameKind::Function(self)
    }
}

impl Frame for NamespaceDecl {
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
    pub inner: T,
    pub stack: Stack<'a>,
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

impl<'a> Anchored<'a, &NamespaceBody> {
    pub fn walk_local_funs(&self, cb: &mut FnMut(Anchored<&FunctionDecl>)) {
        for f in &self.functions {
            cb(self.stack.anchor(f));
        }
    }

    pub fn walk_local_namespaces(&self, cb: &mut FnMut(Anchored<&NamespaceBody>)) {
        for ns in &self.namespaces {
            cb(self.stack.push(ns).anchor(&ns.body));
        }
    }

    pub fn walk_all_funs(&self, cb: &mut FnMut(Anchored<&FunctionDecl>)) {
        self.walk_local_funs(&mut |f| {
            f.walk_all_funs(cb);
            cb(f);
        });
        self.walk_local_namespaces(&mut |ns| ns.walk_all_funs(cb));
    }

    pub fn walk_client_funs(&self, cb: &mut FnMut(Anchored<&FunctionDecl>)) {
        self.walk_local_funs(cb);
        self.walk_local_namespaces(&mut |ns| ns.walk_client_funs(cb));
    }
}

impl<'a> Anchored<'a, &FunctionDecl> {
    pub fn walk_all_funs(&self, cb: &mut FnMut(Anchored<&FunctionDecl>)) {
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

impl<'a> Anchored<'a, &StructDecl> {
    pub fn name(&self) -> &str {
        self.inner.name.text.as_ref()
    }
}

impl<'a> Anchored<'a, &Field> {
    pub fn name(&self) -> &str {
        self.inner.name.text.as_ref()
    }
}
