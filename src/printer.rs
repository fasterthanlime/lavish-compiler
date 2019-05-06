use super::ast;
use super::parser;
use colored::*;

pub struct Visitor<'a> {
    source: &'a parser::Source<'a>,
}

impl<'a> Visitor<'a> {
    pub fn new(source: &'a parser::Source<'a>) -> Self {
        Self { source }
    }

    pub fn visit<T>(&self, v: T)
    where
        T: Visitable,
    {
        v.visit(self)
    }
}

pub trait Visitable {
    fn visit<'a>(self, v: &'a Visitor<'a>);
}

impl<'a> Visitable for &ast::Module<'a> {
    fn visit(self, v: &Visitor) {
        {
            use std::collections::HashMap;
            let mut set: HashMap<&str, &ast::NamespaceDecl> = HashMap::new();
            for ns in &self.namespaces {
                let name = &ns.name;
                if let Some(old) = set.insert(name, &ns) {
                    v.source.position(&ns.loc).print_colored_message(
                        Color::Red,
                        &format!("{} namespace {} redefined", "error:".red().bold(), name),
                    );
                    v.source
                        .position(&old.loc)
                        .print_message(&format!("first definition was here"));
                }
            }
        }

        for ns in &self.namespaces {
            ns.visit(v);
        }
    }
}

impl<'a> Visitable for &ast::NamespaceDecl<'a> {
    fn visit(self, v: &Visitor) {
        v.source.position(&self.loc).print_message(&format!(
            "namespace {}{}",
            self.name.yellow(),
            format_comment(&self.comment),
        ));
        for ns in &self.namespaces {
            ns.visit(v);
        }
        for s in &self.structs {
            s.visit(v);
        }
        for f in &self.functions {
            f.visit(v);
        }
    }
}

impl<'a> Visitable for &ast::StructDecl<'a> {
    fn visit(self, v: &Visitor) {
        v.source.position(&self.loc).print_message(&format!(
            "struct {}{}",
            self.name.yellow(),
            format_comment(&self.comment),
        ));
        for p in &self.fields {
            p.visit(v);
        }
    }
}

impl<'a> Visitable for &ast::FunctionDecl<'a> {
    fn visit(self, v: &Visitor) {
        v.source.position(&self.loc).print_message(&format!(
            "function {}{}",
            self.name.yellow(),
            format_comment(&self.comment),
        ));
        for p in &self.params {
            p.visit(v);
        }
        for p in &self.results {
            p.visit(v);
        }
    }
}

impl<'a> Visitable for &ast::Field<'a> {
    fn visit(self, v: &Visitor) {
        v.source.position(&self.loc).print_message(&format!(
            "field {}, of type {}{}",
            self.name.yellow(),
            self.typ.green(),
            format_comment(&self.comment),
        ));
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
    let v = Visitor::new(source);
    v.visit(module);
}
