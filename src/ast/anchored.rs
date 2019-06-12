use crate::ast::nodes::*;
use std::fmt;

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

    pub fn lookup_struct(&self, mode: LookupMode, down: &[&'a str]) -> Option<RelativePath<'a>> {
        use log::*;
        debug!(
            "lookup_struct({} frames, down = {:?})",
            self.frames.len(),
            down
        );

        if down.is_empty() {
            return None;
        }
        let name = down[0];
        let rest = &down[1..];
        debug!("name = {}, rest = {:?}", name, rest);

        let frames = match mode {
            LookupMode::Strict => &self.frames[self.len() - 1..],
            LookupMode::Relaxed => &self.frames[..],
        };

        for (i, frame) in frames.iter().rev().enumerate() {
            if let Some(body) = frame.body() {
                let mut symbol = None;

                if symbol.is_none() {
                    for s in &body.structs {
                        if s.name.text == name {
                            symbol = Some(Symbol::Struct(&s))
                        }
                    }
                }

                if symbol.is_none() {
                    for ns in &body.namespaces {
                        if ns.name.text == name {
                            symbol = Some(Symbol::Namespace(&ns))
                        }
                    }
                }

                if let Some(symbol) = symbol {
                    debug!("Found match, rest = {:?}", rest);
                    if rest.is_empty() {
                        return Some(RelativePath {
                            up: i,
                            down: vec![name],
                            symbol,
                        });
                    } else {
                        match symbol {
                            Symbol::Namespace(ns) => {
                                debug!("First part of the path resolved to a namespace ({:?}), looking up rest in it", ns.name.text);
                                let stack = self.push(ns);
                                if let Some(mut path) =
                                    stack.lookup_struct(LookupMode::Strict, rest)
                                {
                                    debug!("Sub-lookup did resolve with symbol {:?}", path.symbol);
                                    let mut down = vec![name];
                                    down.append(&mut path.down);
                                    return Some(RelativePath {
                                        up: i,
                                        down,
                                        symbol: path.symbol,
                                    });
                                }
                            }
                            _ => {
                                debug!("Expected first part of a path to resolve to a namespace but found {:?} instead", symbol);
                                return None;
                            }
                        }
                    }
                }
            }
        }
        None
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum LookupMode {
    // Allow looking up in grandparent scopes and higher
    Relaxed,
    // Looks up only in current scope
    Strict,
}

#[derive(Debug)]
pub struct RelativePath<'a> {
    // number of 'supers'
    pub up: usize,

    // namespaces to travel down
    pub down: Vec<&'a str>,

    // symbol that was resolved
    pub symbol: Symbol<'a>,
}

pub enum Symbol<'a> {
    Namespace(&'a NamespaceDecl),
    Struct(&'a StructDecl),
}

impl<'a> fmt::Debug for Symbol<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Symbol::Namespace(node) => write!(f, "Namespace({:?})", node.name.text),
            Symbol::Struct(node) => write!(f, "Struct({:?})", node.name.text),
        }
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
