use super::parser::Span;
use log::*;
use simple_error::SimpleError;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

pub const LAVISH_EXT: &str = ".lavish";
pub const VENDOR_DIR: &str = "lavish-vendor";

#[derive(Debug, Clone)]
pub struct Rules {
    pub loc: Span,
    pub target: Target,
    pub builds: Vec<Build>,
}

impl Rules {
    pub fn new(loc: Span, target: Target, builds: Vec<Build>) -> Self {
        Self {
            loc,
            target,
            builds,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Build {
    pub name: Identifier,
    pub from: Option<FromDirective>,
}

#[derive(Debug, Clone)]
pub struct FromDirective {
    pub path: StringLiteral,
}

#[derive(Debug, Clone)]
pub struct StringLiteral {
    pub loc: Span,
    pub value: String,
}

#[derive(Debug, Clone)]
pub enum Target {
    Rust(RustTarget),
    Go(GoTarget),
    TypeScript(TypeScriptTarget),
}

#[derive(Debug, Clone)]
pub struct RustTarget {}

#[derive(Debug, Clone)]
pub struct GoTarget {}

#[derive(Debug, Clone)]
pub struct TypeScriptTarget {}

#[derive(Debug, Clone)]
pub struct Workspace {
    pub dir: PathBuf,
    pub rules: Rules,
    pub members: HashMap<String, WorkspaceMember>,
}

impl Workspace {
    pub fn resolve(&self, name: &str) -> Result<PathBuf, SimpleError> {
        let source_name = format!("{}{}", name, LAVISH_EXT);

        let self_path = self.dir.join(&source_name);
        debug!("Trying self path {:?}", self_path);
        if self_path.exists() {
            return Ok(self_path);
        }

        let vendor_path = self.dir.join(VENDOR_DIR).join(&source_name);
        debug!("Trying vendor path {:?}", vendor_path);
        if vendor_path.exists() {
            return Ok(vendor_path);
        }

        Err(SimpleError::new(format!(
            "{} not found. Try running `lavish fetch`",
            name
        )))
    }
}

#[derive(Debug, Clone)]
pub struct WorkspaceMember {
    pub name: String,
    pub build: Option<Build>,
    pub imports: Vec<Import>,
    pub schema: Option<Schema>,
}

#[derive(Debug, Clone)]
pub struct Schema {
    pub loc: Span,
    pub imports: Vec<Import>,
    pub root: NamespaceDecl,
}

impl Schema {
    pub fn new(loc: Span, imports: Vec<Import>, root: NamespaceDecl) -> Self {
        Schema { loc, imports, root }
    }
}

#[derive(Debug, Clone)]
pub struct Import {
    pub name: Identifier,
    pub from: Option<FromDirective>,
}

#[derive(Debug, Clone)]
pub struct Identifier {
    pub span: Span,
    pub text: String,
}

#[derive(Debug, Clone)]
pub struct NamespaceDecl {
    pub loc: Span,
    pub comment: Option<Comment>,
    pub name: Identifier,
    pub functions: Vec<FunctionDecl>,
    pub structs: Vec<StructDecl>,
    pub namespaces: Vec<NamespaceDecl>,
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

#[derive(Debug, Clone)]
pub enum NamespaceItem {
    Function(FunctionDecl),
    Struct(StructDecl),
    Namespace(NamespaceDecl),
}

#[derive(Debug, Clone)]
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

#[derive(Debug, Clone)]
pub struct Field {
    pub loc: Span,
    pub comment: Option<Comment>,
    pub name: Identifier,
    pub typ: Type,
}

#[derive(Debug, Clone)]
pub struct Type {
    pub span: Span,
    pub kind: TypeKind,
}

impl Type {
    pub fn text(&self) -> &str {
        self.span.slice()
    }
}

#[derive(Debug, Clone)]
pub enum TypeKind {
    User,
    Base(BaseType),
    Array(ArrayType),
    Option(OptionType),
    Map(MapType),
}

#[derive(Debug, Clone)]
pub enum BaseType {
    Bool,
    Int32,
    Int64,
    UInt32,
    UInt64,
    Float32,
    Float64,
    String,
    Bytes,
    Timestamp,
}

#[derive(Debug, Clone)]
pub struct ArrayType {
    pub inner: Box<Type>,
}

#[derive(Debug, Clone)]
pub struct OptionType {
    pub inner: Box<Type>,
}

#[derive(Debug, Clone)]
pub struct MapType {
    pub keys: Box<Type>,
    pub values: Box<Type>,
}

#[derive(Debug, Clone)]
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
