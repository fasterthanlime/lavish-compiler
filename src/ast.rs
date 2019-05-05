use std::collections::HashMap;

#[derive(Debug)]
pub struct NamespaceDecl {
    comment: Option<Comment>,
    name: String,
    functions: HashMap<String, FunctionDecl>,
    structs: HashMap<String, StructDecl>,
    namespaces: HashMap<String, NamespaceDecl>,
}

#[derive(Debug)]
pub enum NamespaceItem {
    Function(FunctionDecl),
    Struct(StructDecl),
    Namespace(NamespaceDecl),
}

#[derive(Debug)]
pub struct FunctionDecl {
    pub comment: Option<Comment>,
    pub modifiers: Vec<FunctionModifier>,
    pub name: String,
    pub params: Vec<Field>,
    pub results: Vec<Field>,
}

#[derive(Debug, Clone)]
pub enum FunctionModifier {
    Server,
    Client,
}

#[derive(Debug)]
pub struct Field {
    pub comment: Option<Comment>,
    pub name: String,
    pub typ: String,
}

#[derive(Debug)]
pub struct StructDecl {
    pub comment: Option<Comment>,
    pub name: String,
}

#[derive(Debug, Clone)]
pub struct Comment {
    pub lines: Vec<String>,
}

impl NamespaceDecl {
    pub fn new(name: &str, comment: Option<Comment>, items: Vec<NamespaceItem>) -> Self {
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

    fn add_item(&mut self, item: NamespaceItem) {
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
