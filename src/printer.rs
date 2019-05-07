use super::ast;
use super::parser;
use colored::*;

struct Visitor {
    indent: usize,
}

impl Visitor {
    fn new() -> Self {
        Self { indent: 0 }
    }

    fn visit<T>(&mut self, v: T)
    where
        T: Visitable,
    {
        self.indent += 1;
        v.visit(self);
        self.indent -= 1;
    }

    fn indent(&self) -> String {
        let indent = match self.indent {
            1 => 0,
            x => x,
        };
        "  ".repeat(indent) + "|"
    }

    fn print(&self, loc: &parser::Span, msg: String) {
        loc.position().diag_info(msg).prefix(&self.indent()).print();
    }
}

trait Visitable {
    fn visit(self, v: &mut Visitor);
}

impl Visitable for &ast::Module {
    fn visit(self, v: &mut Visitor) {
        for ns in &self.namespaces {
            v.visit(ns);
        }
    }
}

impl Visitable for &ast::NamespaceDecl {
    fn visit(self, v: &mut Visitor) {
        v.print(
            &self.loc,
            format!(
                "namespace {}{}",
                self.name.yellow(),
                format_comment(&self.comment),
            ),
        );
        for ns in &self.namespaces {
            v.visit(ns);
        }
        for s in &self.structs {
            v.visit(s);
        }
        for f in &self.functions {
            v.visit(f);
        }
    }
}

impl Visitable for &ast::StructDecl {
    fn visit(self, v: &mut Visitor) {
        v.print(
            &self.loc,
            format!(
                "struct {}{}",
                self.name.yellow(),
                format_comment(&self.comment),
            ),
        );
        for p in &self.fields {
            v.visit(p);
        }
    }
}

impl Visitable for &ast::FunctionDecl {
    fn visit(self, v: &mut Visitor) {
        v.print(
            &self.loc,
            format!(
                "function {}{}",
                self.name.yellow(),
                format_comment(&self.comment),
            ),
        );
        for f in &self.params {
            v.visit(f);
        }
        for f in &self.results {
            v.visit(f);
        }
    }
}

impl Visitable for &ast::Field {
    fn visit(self, v: &mut Visitor) {
        v.print(
            &self.loc,
            format!(
                "field {}, of type {}{}",
                self.name.yellow(),
                self.typ.green(),
                format_comment(&self.comment),
            ),
        );
    }
}

fn format_comment(comment: &Option<ast::Comment>) -> ColoredString {
    let mut result = ColoredString::default();
    if let Some(comment) = comment.as_ref() {
        result = format!(
            " — {}",
            comment.lines.iter().map(|x| x.clone()).collect::<String>()
        )
        .blue();
    };
    result
}

pub fn print(module: &ast::Module) {
    let mut v = Visitor::new();
    v.visit(module);
}
