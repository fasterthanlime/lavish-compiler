use crate::ast::nodes::*;

pub trait Frame {
    fn name(&self) -> String;
    fn kind(&self) -> FrameKind;
}

pub enum FrameKind<'a> {
    Namespace(&'a NamespaceDecl),
    Function(&'a FunctionDecl),
}

impl Frame for FunctionDecl {
    fn name(&self) -> String {
        self.name.text.clone()
    }

    fn kind(&self) -> FrameKind {
        FrameKind::Function(self)
    }
}

impl Frame for NamespaceDecl {
    fn name(&self) -> String {
        self.name.text.clone()
    }

    fn kind(&self) -> FrameKind {
        FrameKind::Namespace(self)
    }
}

#[derive(Clone)]
pub struct Stack<'a> {
    pub(crate) frames: Vec<&'a Frame>,
}

#[allow(non_snake_case)]
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
}

#[derive(Clone)]
pub struct Anchored<'a, T> {
    pub(crate) inner: T,
    pub(crate) stack: Stack<'a>,
}

impl<'a, T> std::ops::Deref for Anchored<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

pub fn filter_funs<Predicate, I>(
    predicate: Predicate,
    mut cb: I,
) -> impl FnMut(Anchored<&FunctionDecl>)
where
    Predicate: Fn(&Anchored<&FunctionDecl>) -> bool + 'static,
    I: FnMut(Anchored<&FunctionDecl>),
{
    move |f| {
        if predicate(&f) {
            cb(f)
        }
    }
}

pub fn filter_fun_side<I>(side: Side, cb: I) -> impl FnMut(Anchored<&FunctionDecl>)
where
    I: FnMut(Anchored<&FunctionDecl>),
{
    filter_funs(move |f| f.side == side, cb)
}

impl<'a> Anchored<'a, &NamespaceBody> {
    pub fn for_each_fun(&self, cb: &mut FnMut(Anchored<&FunctionDecl>)) {
        for f in &self.functions {
            cb(self.stack.anchor(f));
        }
    }

    pub fn for_each_namespace(&self, cb: &mut FnMut(Anchored<&NamespaceBody>)) {
        for ns in &self.namespaces {
            cb(self.stack.push(ns).anchor(&ns.body));
        }
    }

    pub fn for_each_fun_of_schema(&self, cb: &mut FnMut(Anchored<&FunctionDecl>)) {
        self.for_each_fun(&mut |f| {
            f.for_each_fun_of_schema(cb);
            cb(f);
        });
        self.for_each_namespace(&mut |ns| ns.for_each_fun_of_schema(cb));
    }

    pub fn for_each_fun_of_interface(&self, cb: &mut FnMut(Anchored<&FunctionDecl>)) {
        self.for_each_fun(cb);
        self.for_each_namespace(&mut |ns| ns.for_each_fun_of_interface(cb));
    }
}

impl<'a> Anchored<'a, &FunctionDecl> {
    pub fn for_each_fun_of_schema(&self, cb: &mut FnMut(Anchored<&FunctionDecl>)) {
        if let Some(body) = self.body.as_ref() {
            self.stack
                .push(self.inner)
                .anchor(body)
                .for_each_fun_of_schema(cb);
        }
    }

    pub fn names(&self) -> Vec<String> {
        let mut names = self.stack.names();
        names.push(self.name().into());
        names
    }

    pub fn method(&self) -> String {
        self.names().join(".")
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
