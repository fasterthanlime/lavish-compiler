use super::super::ast;
use super::Error;
use heck::{CamelCase, MixedCase, SnakeCase};
use indexmap::IndexMap;
use std::cell::RefCell;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;

const INDENT_WIDTH: usize = 4;

struct Output {
    writer: RefCell<BufWriter<File>>,
}

impl Output {
    fn new(file: File) -> Self {
        Self {
            writer: RefCell::new(BufWriter::new(file)),
        }
    }

    fn write_indented(&self, indent: usize, line: &str) {
        let mut w = self.writer.borrow_mut();
        for _ in 0..indent {
            write!(w, "{}", " ").unwrap();
        }
        write!(w, "{}\n", line).unwrap();
    }
}

struct Context<'a> {
    namespaces: IndexMap<&'a str, Namespace<'a>>,
    output: Output,
}

struct Scope<'a> {
    output: &'a Output,
    indent: usize,
}

impl<'a> Context<'a> {
    fn new(file: File) -> Self {
        Self {
            namespaces: IndexMap::new(),
            output: Output::new(file),
        }
    }

    fn visit_toplevel_ns(&mut self, decl: &'a ast::NamespaceDecl) {
        let k = decl.name.text.as_ref();

        let mut ns = Namespace::new(decl);
        self.visit_ns("", &mut ns);

        if let Some(original_ns) = self.namespaces.get_mut(k) {
            original_ns.merge(ns)
        } else {
            self.namespaces.insert(k, ns);
        }
    }

    fn visit_ns(&mut self, prefix: &str, ns: &mut Namespace<'a>) {
        let decl = &ns.decl;
        let prefix = format!("{}{}.", prefix, ns.name());

        for decl in &decl.functions {
            let ff = Fun::<'a> {
                decl,
                full_name: format!("{}{}", prefix, decl.name.text),
            };
            ns.funs.insert(&decl.name.text, ff);
        }

        for decl in &decl.namespaces {
            let mut child = Namespace::new(decl);
            self.visit_ns(&prefix, &mut child);
            ns.children.insert(decl.name.text.as_ref(), child);
        }
    }

    fn funs(&self) -> Box<Iterator<Item = &'a Fun> + 'a> {
        Box::new(self.namespaces.values().map(|ns| ns.funs()).flatten())
    }
}

trait ScopeLike<'a> {
    fn line(&self, line: &str);
    fn scope<'b: 'a>(&'b self) -> Scope<'b>;

    fn comment(&self, comment: &Option<ast::Comment>) {
        if let Some(comment) = comment.as_ref() {
            for line in &comment.lines {
                self.line(&format!("// {}", line))
            }
        }
    }

    fn def_struct<'b: 'a>(&'b self, name: &str, f: &Fn(&ScopeLike)) {
        self.line("#[derive(Serialize, Deserialize, Debug)]");
        self.line(&format!("pub struct {} {{", name));
        {
            let s = self.scope();
            f(&s);
        }
        self.line("}");
    }
}

impl<'a> ScopeLike<'a> for Context<'a> {
    fn line(&self, line: &str) {
        self.output.write_indented(0, line)
    }

    fn scope<'b: 'a>(&'b self) -> Scope<'b> {
        Scope {
            output: &self.output,
            indent: INDENT_WIDTH,
        }
    }
}

impl<'a> ScopeLike<'a> for Scope<'a> {
    fn line(&self, line: &str) {
        self.output.write_indented(self.indent, line)
    }

    fn scope<'b: 'a>(&'b self) -> Scope<'b> {
        Scope {
            output: &self.output,
            indent: self.indent + INDENT_WIDTH,
        }
    }
}

struct Namespace<'a> {
    decl: &'a ast::NamespaceDecl,
    children: IndexMap<&'a str, Namespace<'a>>,

    funs: IndexMap<&'a str, Fun<'a>>,
}

impl<'a> Namespace<'a> {
    fn new(decl: &'a ast::NamespaceDecl) -> Self {
        Namespace {
            decl,
            children: IndexMap::new(),
            funs: IndexMap::new(),
        }
    }

    fn funs(&self) -> Box<Iterator<Item = &'a Fun> + 'a> {
        Box::new(
            self.children
                .values()
                .map(|ns| ns.funs())
                .flatten()
                .chain(self.funs.values()),
        )
    }

    fn merge(&mut self, rhs: Self) {
        for (k, v) in rhs.children {
            if let Some(sv) = self.children.get_mut(k) {
                sv.merge(v)
            } else {
                self.children.insert(k, v);
            }
        }

        for (k, v) in rhs.funs {
            self.funs.insert(k, v);
        }
    }

    fn name(&self) -> &'a str {
        &self.decl.name.text
    }
}

struct Fun<'a> {
    #[allow(unused)]
    decl: &'a ast::FunctionDecl,
    full_name: String,
}

impl<'a> Fun<'a> {
    fn rpc_name(&self) -> String {
        let tokens: Vec<_> = self.full_name.split(".").collect();
        let last = tokens.len() - 1;
        tokens
            .iter()
            .enumerate()
            .map(|(i, x)| {
                if i == last {
                    x.to_camel_case()
                } else {
                    x.to_mixed_case()
                }
            })
            .collect::<Vec<_>>()
            .join(".")
    }

    fn qualified_mod_name(&self) -> String {
        self.rpc_name().replace(".", "::")
    }

    fn mod_name(&self) -> String {
        self.decl.name.text.to_snake_case()
    }
}

type Result = std::result::Result<(), Error>;

pub fn codegen<'a>(modules: &'a Vec<ast::Module>) -> Result {
    let p = Path::new("output/out.rs");
    std::fs::create_dir_all(p.parent().unwrap())?;
    let mut out = File::create(p).unwrap();

    write!(
        out,
        "{}",
        r#"
#[derive(Debug)]
enum Message {
    Request {
        parent: Option<u32>,
        id: u32,
        params: Params,
    },
    #[allow(unused)]
    Response {
        id: u32,
        error: Option<String>,
        results: Results,
    },
}
"#
    )?;

    let mut ctx = Context::new(out);
    for module in modules {
        for decl in &module.namespaces {
            ctx.visit_toplevel_ns(decl);
        }
    }

    fn visit_ns<'a>(ctx: &'a ScopeLike<'a>, ns: &Namespace) -> Result {
        ctx.line(&format!("pub mod {} {{", ns.name()));
        {
            let ctx = ctx.scope();
            for (_, ns) in &ns.children {
                visit_ns(&ctx, ns)?;
            }

            for (_, fun) in &ns.funs {
                ctx.comment(&fun.decl.comment);
                ctx.line(&format!("pub mod {} {{", fun.mod_name()));

                {
                    let ctx = ctx.scope();
                    ctx.line("use serde_derive::*;");
                    ctx.line("");

                    ctx.def_struct("Params", &|ctx| {
                        for f in &fun.decl.params {
                            ctx.line(&format!("{}: {},", f.name.text, f.typ));
                        }
                    });
                    ctx.line("");

                    ctx.def_struct("Results", &|ctx| {
                        for f in &fun.decl.results {
                            ctx.line(&format!("{}: {},", f.name.text, f.typ));
                        }
                    });
                }
                ctx.line("}");
                ctx.line("");
            }
        }
        ctx.line("}");
        ctx.line("");
        Ok(())
    }

    for (_, ns) in &ctx.namespaces {
        ctx.line("");
        visit_ns(&ctx, ns)?;
    }

    for f in ctx.funs() {
        ctx.line(&format!("// Should list {:?} in params", f.rpc_name()));
    }

    println!("All done!");

    Ok(())
}
