#[derive(Debug, Clone)]
pub struct Loc<'a> {
    pub slice: &'a str,
}

#[derive(Debug)]
pub struct Module<'a> {
    pub namespaces: Vec<NamespaceDecl<'a>>,
}

pub struct Module2 {
    pub source: std::rc::Rc<super::parser::Source>,
}

impl<'a> Module<'a> {
    pub fn new(namespaces: Vec<NamespaceDecl<'a>>) -> Self {
        Self { namespaces }
    }
}

#[derive(Debug)]
pub struct NamespaceDecl<'a> {
    pub loc: Loc<'a>,
    pub comment: Option<Comment>,
    pub name: String,
    pub functions: Vec<FunctionDecl<'a>>,
    pub structs: Vec<StructDecl<'a>>,
    pub namespaces: Vec<NamespaceDecl<'a>>,
}

#[derive(Debug)]
pub enum NamespaceItem<'a> {
    Function(FunctionDecl<'a>),
    Struct(StructDecl<'a>),
    Namespace(NamespaceDecl<'a>),
}

#[derive(Debug)]
pub struct FunctionDecl<'a> {
    pub loc: Loc<'a>,
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
    pub loc: Loc<'a>,
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
    pub fn new(
        name: &str,
        loc: Loc<'a>,
        comment: Option<Comment>,
        items: Vec<NamespaceItem<'a>>,
    ) -> Self {
        let mut ns = NamespaceDecl {
            name: name.into(),
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

    fn add_item(&mut self, item: NamespaceItem<'a>) {
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
