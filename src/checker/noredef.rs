use super::ast;
use super::parser;
use super::Error;
use colored::*;

pub struct Visitor<'a> {
    num_errors: i64,
    source: &'a parser::Source<'a>,
}

impl<'a> Visitor<'a> {
    pub fn visit<'b, T>(&'b mut self, v: T)
    where
        T: Visitable<'b>,
    {
        v.visit(self)
    }
}

pub trait Visitable<'a> {
    fn visit(self, v: &'a mut Visitor);
}

impl<'a> Visitable<'a> for &ast::Module<'a> {
    fn visit(self, v: &'a mut Visitor) {
        {
            use std::collections::HashMap;
            let mut set: HashMap<&str, &ast::NamespaceDecl> = HashMap::new();
            for ns in &self.namespaces {
                let name = &ns.name;
                if let Some(old) = set.insert(name, &ns) {
                    v.num_errors += 1;
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

impl<'a> Visitable<'a> for &ast::NamespaceDecl<'a> {
    fn visit(self, v: &'a mut Visitor) {
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

impl<'a> Visitable<'a> for &ast::StructDecl<'a> {
    fn visit(self, v: &'a mut Visitor) {
        for p in &self.fields {
            p.visit(v);
        }
    }
}

impl<'a> Visitable<'a> for &ast::FunctionDecl<'a> {
    fn visit(self, v: &'a mut Visitor) {
        for p in &self.params {
            p.visit(v);
        }
        for p in &self.results {
            p.visit(v);
        }
    }
}

impl<'a> Visitable<'a> for &ast::Field<'a> {
    fn visit(self, _v: &'a mut Visitor) {}
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
