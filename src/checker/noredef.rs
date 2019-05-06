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

    fn check_dupes<'b, T>(&mut self, kind: &str, items: &'b Vec<T>)
    where
        T: Named<'b>,
    {
        let mut set: HashMap<&str, &T> = HashMap::new();
        for item in items {
            let name = item.name();
            if let Some(old) = set.insert(name, item) {
                self.num_errors += 1;
                self.source.position(item.loc()).print_colored_message(
                    Color::Red,
                    &format!("{} {} {} redefined", "error:".red().bold(), kind, name),
                );
                self.source
                    .position(old.loc())
                    .print_message(&format!("first definition was here"));
            }
        }
    }
}

trait Visitable {
    fn visit(self, v: &mut Visitor);
}

trait Named<'a> {
    fn name(&'a self) -> &'a str;
    fn loc(&'a self) -> &'a ast::Loc<'a>;
}

macro_rules! impl_named {
    ($x:ty) => {
        impl<'a> Named<'a> for $x {
            fn name(&'a self) -> &'a str {
                &self.name
            }
            fn loc(&'a self) -> &'a ast::Loc<'a> {
                &self.loc
            }
        }
    };
}

impl_named!(ast::NamespaceDecl<'a>);
impl_named!(ast::StructDecl<'a>);
impl_named!(ast::FunctionDecl<'a>);
impl_named!(ast::Field<'a>);

impl<'a> Visitable for &ast::Module<'a> {
    fn visit(self, v: &mut Visitor) {
        v.check_dupes("namespace", &self.namespaces);
        for ns in &self.namespaces {
            v.visit(ns);
        }
    }
}

impl<'a> Visitable for &ast::NamespaceDecl<'a> {
    fn visit(self, v: &mut Visitor) {
        v.check_dupes("namespace", &self.namespaces);
        for ns in &self.namespaces {
            v.visit(ns);
        }
        v.check_dupes("struct", &self.structs);
        for s in &self.structs {
            v.visit(s);
        }
        v.check_dupes("function", &self.functions);
        for f in &self.functions {
            v.visit(f);
        }
    }
}

impl<'a> Visitable for &ast::StructDecl<'a> {
    fn visit(self, v: &mut Visitor) {
        v.check_dupes("field", &self.fields);
        for p in &self.fields {
            v.visit(p);
        }
    }
}

impl<'a> Visitable for &ast::FunctionDecl<'a> {
    fn visit(self, v: &mut Visitor) {
        v.check_dupes("param", &self.params);
        for p in &self.params {
            v.visit(p);
        }
        v.check_dupes("result", &self.results);
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
