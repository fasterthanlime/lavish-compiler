use crate::parser::Span;
use crc::crc32::checksum_castagnoli;
use log::*;
use simple_error::SimpleError;
use std::collections::HashMap;
use std::fmt;
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
pub struct RustTarget {
    pub wrapper: RustTargetWrapper,
}

impl RustTarget {
    pub fn new(items: Vec<RustTargetItem>) -> Self {
        let mut s = Self {
            wrapper: RustTargetWrapper::Mod,
        };
        for item in items {
            match item {
                RustTargetItem::Wrapper(wrapper) => {
                    s.wrapper = wrapper;
                }
            }
        }
        s
    }
}

#[derive(Debug, Clone)]
pub enum RustTargetWrapper {
    None,
    Mod,
    Lib,
}

pub enum RustTargetItem {
    Wrapper(RustTargetWrapper),
}

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
    pub body: NamespaceBody,
}

impl Schema {
    pub fn new(loc: Span, imports: Vec<Import>, body: NamespaceBody) -> Self {
        Schema { loc, imports, body }
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
}

impl Identifier {
    pub fn text(&self) -> &str {
        self.span.slice()
    }
}

#[derive(Debug, Clone)]
pub struct NamespaceDecl {
    pub loc: Span,
    pub comment: Option<Comment>,
    pub name: Identifier,
    pub body: NamespaceBody,
}

#[derive(Debug, Clone)]
pub struct NamespaceBody {
    pub functions: Vec<FunctionDecl>,
    pub structs: Vec<StructDecl>,
    pub enums: Vec<EnumDecl>,
    pub namespaces: Vec<NamespaceDecl>,
}

impl NamespaceDecl {
    pub fn new(name: Identifier, loc: Span, comment: Option<Comment>, body: NamespaceBody) -> Self {
        Self {
            name,
            loc,
            comment,
            body,
        }
    }
}

impl NamespaceBody {
    pub fn new(items: Vec<NamespaceItem>) -> Self {
        let mut bod = Self {
            functions: Vec::new(),
            structs: Vec::new(),
            enums: Vec::new(),
            namespaces: Vec::new(),
        };
        for item in items {
            bod.add_item(item);
        }
        bod
    }

    fn add_item(&mut self, item: NamespaceItem) {
        match item {
            NamespaceItem::Function(i) => {
                self.functions.push(i);
            }
            NamespaceItem::Struct(i) => {
                self.structs.push(i);
            }
            NamespaceItem::Enum(i) => {
                self.enums.push(i);
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
    Enum(EnumDecl),
    Namespace(NamespaceDecl),
}

#[derive(Debug, Clone)]
pub struct FunctionDecl {
    pub loc: Span,
    pub comment: Option<Comment>,
    pub name: Identifier,
    pub params: Vec<Field>,
    pub results: Vec<Field>,
    pub body: Option<NamespaceBody>,
    pub kind: Kind,
    pub side: Side,
}

#[derive(PartialEq, Eq, Debug, Clone, Copy)]
pub enum Side {
    Client,
    Server,
}

impl fmt::Display for Side {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(match self {
            Side::Client => "client",
            Side::Server => "server",
        })
    }
}

impl Side {
    pub fn other(self) -> Self {
        match self {
            Side::Client => Side::Server,
            Side::Server => Side::Client,
        }
    }
}

#[derive(PartialEq, Eq, Debug, Clone, Copy)]
pub enum Kind {
    Request,
    Notification,
}

impl FunctionDecl {}

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
    U8,
    I8,
    U16,
    I16,
    U32,
    I32,
    U64,
    I64,
    F32,
    F64,
    String,
    Data,
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
pub struct EnumDecl {
    pub loc: Span,
    pub comment: Option<Comment>,
    pub name: Identifier,
    pub variants: Vec<EnumVariant>,
}

#[derive(Debug, Clone)]
pub struct EnumVariant {
    pub loc: Span,
    pub comment: Option<Comment>,
    pub name: Identifier,
}

impl EnumVariant {
    pub fn hash(&self) -> u32 {
        checksum_castagnoli(self.name.text().as_bytes())
    }
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
