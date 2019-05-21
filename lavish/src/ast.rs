use super::parser::Span;
use std::collections::HashSet;

#[derive(Debug)]
pub struct Module {
    pub loc: Span,
    pub namespaces: Vec<NamespaceDecl>,
}

impl Module {
    pub fn new(loc: Span, namespaces: Vec<NamespaceDecl>) -> Self {
        Self { loc, namespaces }
    }
}

#[derive(Debug, Clone)]
pub struct Identifier {
    pub span: Span,
    pub text: String,
}

#[derive(Debug)]
pub struct NamespaceDecl {
    pub loc: Span,
    pub comment: Option<Comment>,
    pub name: Identifier,
    pub functions: Vec<FunctionDecl>,
    pub structs: Vec<StructDecl>,
    pub namespaces: Vec<NamespaceDecl>,
}

#[derive(Debug)]
pub enum NamespaceItem {
    Function(FunctionDecl),
    Struct(StructDecl),
    Namespace(NamespaceDecl),
}

#[derive(Debug)]
pub struct FunctionDecl {
    pub loc: Span,
    pub comment: Option<Comment>,
    pub modifiers: HashSet<FunctionModifier>,
    pub name: Identifier,
    pub params: Vec<Field>,
    pub results: Vec<Field>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum FunctionModifier {
    Server,
    Client,
    Notification,
}

#[derive(Debug)]
pub struct Field {
    pub loc: Span,
    pub comment: Option<Comment>,
    pub name: Identifier,
    pub typ: String,
}

#[derive(Debug)]
pub struct StructDecl {
    pub loc: Span,
    pub comment: Option<Comment>,
    pub name: Identifier,
    pub fields: Vec<Field>,
}

#[derive(Debug, Clone)]
pub struct Comment {
    pub lines: Vec<String>,
}

impl std::default::Default for Comment {
    fn default() -> Self {
        Comment { lines: Vec::new() }
    }
}

impl NamespaceDecl {
    pub fn new(
        name: Identifier,
        loc: Span,
        comment: Option<Comment>,
        items: Vec<NamespaceItem>,
    ) -> Self {
        let mut ns = NamespaceDecl {
            name,
            loc,
            comment,
            functions: Vec::new(),
            structs: Vec::new(),
            namespaces: Vec::new(),
        };
        for item in items {
            ns.add_item(item)
        }
        ns
    }

    fn add_item(&mut self, item: NamespaceItem) {
        match item {
            NamespaceItem::Function(i) => {
                self.functions.push(i);
            }
            NamespaceItem::Struct(i) => {
                self.structs.push(i);
            }
            NamespaceItem::Namespace(i) => {
                self.namespaces.push(i);
            }
        };
    }
}
