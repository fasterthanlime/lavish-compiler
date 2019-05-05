use std::collections::HashMap;

#[derive(Debug)]
pub struct NamespaceDecl {
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
    pub name: String,
    pub args: Vec<FunctionArg>,
}

#[derive(Debug)]
pub struct FunctionArg {
    pub name: String,
    pub typ: String,
}

#[derive(Debug)]
pub struct StructDecl {
    pub name: String,
}

impl NamespaceDecl {
    pub fn new(name: &str, items: Vec<NamespaceItem>) -> Self {
        let mut ns = NamespaceDecl {
            name: name.into(),
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
