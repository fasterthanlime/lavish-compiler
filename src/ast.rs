use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct Loc<'a> {
    pub slice: &'a str,
}

#[derive(Debug)]
pub struct NamespaceDecl<'a> {
    pub comment: Option<Comment>,
    pub name: String,
    pub functions: HashMap<String, FunctionDecl<'a>>,
    pub structs: HashMap<String, StructDecl<'a>>,
    pub namespaces: HashMap<String, NamespaceDecl<'a>>,
}

#[derive(Debug)]
pub enum NamespaceItem<'a> {
    Function(FunctionDecl<'a>),
    Struct(StructDecl<'a>),
    Namespace(NamespaceDecl<'a>),
}

#[derive(Debug)]
pub struct FunctionDecl<'a> {
    pub comment: Option<Comment>,
    pub modifiers: Vec<FunctionModifier>,
    pub name: String,
    pub params: Vec<Field<'a>>,
    pub results: Vec<Field<'a>>,
}

#[derive(Debug, Clone)]
pub enum FunctionModifier {
    Server,
    Client,
}

#[derive(Debug)]
pub struct Field<'a> {
    pub loc: Loc<'a>,
    pub comment: Option<Comment>,
    pub name: String,
    pub typ: String,
}

#[derive(Debug)]
pub struct StructDecl<'a> {
    pub comment: Option<Comment>,
    pub name: String,
    pub fields: Vec<Field<'a>>,
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

impl<'a> NamespaceDecl<'a> {
    pub fn new(name: &str, comment: Option<Comment>, items: Vec<NamespaceItem<'a>>) -> Self {
        let mut ns = NamespaceDecl {
            name: name.into(),
            comment,
            functions: HashMap::new(),
            structs: HashMap::new(),
            namespaces: HashMap::new(),
        };
        for item in items {
            ns.add_item(item)
        }
        ns
    }

    fn add_item(&mut self, item: NamespaceItem<'a>) {
        match item {
            NamespaceItem::Function(i) => {
                self.functions.insert(i.name.clone(), i);
            }
            NamespaceItem::Struct(i) => {
                self.structs.insert(i.name.clone(), i);
            }
            NamespaceItem::Namespace(i) => {
                self.namespaces.insert(i.name.clone(), i);
            }
        };
    }
}
