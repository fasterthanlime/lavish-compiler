use super::ast;
use super::parser;
use super::Error;
use colored::*;
use std::collections::HashMap;

struct Visitor<'a> {
    num_errors: i64,
    source: &'a parser::Source<'a>,
}

impl<'a> Visitor<'a> {
    fn visit<T>(&mut self, v: T)
    where
        T: Visitable,
    {
        v.visit(self)
    }
}

trait Visitable {
    fn visit(self, v: &mut Visitor);
}

trait Named<'a> {
    fn name(&'a self) -> &'a str;
    fn loc(&'a self) -> &'a ast::Loc<'a>;
}

fn check_dupes<'a, T>(v: &mut Visitor, kind: &str, items: &'a Vec<T>)
where
    T: Named<'a>,
{
    let mut set: HashMap<&str, &T> = HashMap::new();
    for item in items {
        let name = item.name();
        if let Some(old) = set.insert(name, item) {
            v.num_errors += 1;
            v.source.position(item.loc()).print_colored_message(
                Color::Red,
                &format!("{} {} {} redefined", "error:".red().bold(), kind, name),
            );
            v.source
                .position(old.loc())
                .print_message(&format!("first definition was here"));
        }
    }
}

impl<'a> Named<'a> for ast::NamespaceDecl<'a> {
    fn name(&'a self) -> &'a str {
        &self.name
    }
    fn loc(&'a self) -> &'a ast::Loc<'a> {
        &self.loc
    }
}

impl<'a> Visitable for &ast::Module<'a> {
    fn visit(self, v: &mut Visitor) {
        check_dupes(v, "namespace", &self.namespaces);

        for ns in &self.namespaces {
            v.visit(ns);
        }
    }
}

impl<'a> Visitable for &ast::NamespaceDecl<'a> {
    fn visit(self, v: &mut Visitor) {
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
        for p in &self.fields {
            v.visit(p);
        }
    }
}

impl<'a> Visitable for &ast::FunctionDecl<'a> {
    fn visit(self, v: &mut Visitor) {
        for p in &self.params {
            v.visit(p);
        }
        for p in &self.results {
            v.visit(p);
        }
    }
}

impl<'a> Visitable for &ast::Field<'a> {
    fn visit(self, _v: &mut Visitor) {}
}

pub fn check(source: &parser::Source, module: &ast::Module) -> Result<(), Error> {
    let mut v = Visitor {
        source,
        num_errors: 0,
    };
    v.visit(module);
    if v.num_errors > 0 {
        Err(Error {
            num_errors: v.num_errors,
        })
    } else {
        Ok(())
    }
}
