use super::ast;
use super::parser;
use colored::*;

struct Visitor<'a> {
    indent: usize,
    source: &'a parser::Source<'a>,
}

impl<'a> Visitor<'a> {
    fn new(source: &'a parser::Source<'a>) -> Self {
        Self { indent: 0, source }
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

    fn print(&self, loc: &ast::Loc, msg: &str) {
        let pos = self.source.position(loc);
        pos.print_colored_message_with_prefix(Color::Blue, &self.indent(), msg);
    }
}

trait Visitable {
    fn visit(self, v: &mut Visitor);
}

impl<'a> Visitable for &ast::Module<'a> {
    fn visit(self, v: &mut Visitor) {
        for ns in &self.namespaces {
            v.visit(ns);
        }
    }
}

impl<'a> Visitable for &ast::NamespaceDecl<'a> {
    fn visit(self, v: &mut Visitor) {
        v.print(
            &self.loc,
            &format!(
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

impl<'a> Visitable for &ast::StructDecl<'a> {
    fn visit(self, v: &mut Visitor) {
        v.print(
            &self.loc,
            &format!(
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

impl<'a> Visitable for &ast::FunctionDecl<'a> {
    fn visit(self, v: &mut Visitor) {
        v.print(
            &self.loc,
            &format!(
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

impl<'a> Visitable for &ast::Field<'a> {
    fn visit(self, v: &mut Visitor) {
        v.print(
            &self.loc,
            &format!(
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

pub fn print(source: &parser::Source, module: &ast::Module) {
    let mut v = Visitor::new(source);
    v.visit(module);
}
