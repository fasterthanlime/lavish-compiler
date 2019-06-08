use crate::ast::nodes::*;

pub trait Frame: std::fmt::Debug {
    fn name(&self) -> String;
    fn kind(&self) -> FrameKind;
    fn body(&self) -> Option<&NamespaceBody>;
}

pub enum FrameKind<'a> {
    Schema(&'a Schema),
    Namespace(&'a NamespaceDecl),
    Function(&'a FunctionDecl),
    Synthetic(&'a SyntheticFrame),
}

#[derive(Debug)]
pub struct SyntheticFrame {
    name: String,
}

impl SyntheticFrame {
    pub fn new<N>(name: N) -> Self
    where
        N: Into<String>,
    {
        Self { name: name.into() }
    }
}

impl Frame for SyntheticFrame {
    fn name(&self) -> String {
        self.name.clone()
    }

    fn kind(&self) -> FrameKind {
        FrameKind::Synthetic(self)
    }

    fn body(&self) -> Option<&NamespaceBody> {
        None
    }
}

impl Frame for Schema {
    fn name(&self) -> String {
        "<schema>".into()
    }

    fn kind(&self) -> FrameKind {
        FrameKind::Schema(self)
    }

    fn body(&self) -> Option<&NamespaceBody> {
        Some(&self.body)
    }
}

impl Frame for FunctionDecl {
    fn name(&self) -> String {
        self.name.text.clone()
    }

    fn kind(&self) -> FrameKind {
        FrameKind::Function(self)
    }

    fn body(&self) -> Option<&NamespaceBody> {
        self.body.as_ref()
    }
}

impl Frame for NamespaceDecl {
    fn name(&self) -> String {
        self.name.text.clone()
    }

    fn kind(&self) -> FrameKind {
        FrameKind::Namespace(self)
    }

    fn body(&self) -> Option<&NamespaceBody> {
        Some(&self.body)
    }
}

#[derive(Clone)]
pub struct Stack<'a> {
    pub(crate) frames: Vec<&'a Frame>,
}

#[allow(non_snake_case)]
impl<'a> Stack<'a> {
    pub fn new(frame: &'a Frame) -> Self {
        Self {
            frames: vec![frame],
        }
    }

    pub fn push(&self, frame: &'a Frame) -> Self {
        let mut frames = self.frames.clone();
        frames.push(frame);
        Self { frames }
    }

    pub fn len(&self) -> usize {
        self.frames.len()
    }

    pub fn anchor<T>(&self, inner: T) -> Anchored<T> {
        Anchored {
            stack: self.clone(),
            inner,
        }
    }

    pub fn names(&self) -> Vec<String> {
        self.frames
            .iter()
            .map(|f| match f.kind() {
                FrameKind::Schema(_) => None,
                FrameKind::Synthetic(_) => None,
                _ => Some(f.name()),
            })
            .filter_map(|x| x)
            .collect()
    }

    pub fn lookup_struct(&self, name: &str) -> Option<RelativePath> {
        for (i, frame) in self.frames.iter().rev().enumerate() {
            if let Some(body) = frame.body() {
                for s in &body.structs {
                    if s.name.text == name {
                        return Some(RelativePath {
                            up: i,
                            down: vec![name.into()],
                        });
                    }
                }
            }
        }
        None
    }
}

#[derive(Debug)]
pub struct RelativePath {
    // number of 'supers'
    pub up: usize,
    // namespaces to travel down
    pub down: Vec<String>,
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
