use super::ast;
use super::parser;
use super::Error;
use colored::*;
use std::collections::HashMap;

struct Visitor {
    num_errors: i64,
}

impl Visitor {
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
                item.loc()
                    .position()
                    .diag_err(format!(
                        "{} {} {} redefined",
                        "error:".red().bold(),
                        kind,
                        name
                    ))
                    .print();
                old.loc()
                    .position()
                    .diag_info("first definition was here".into())
                    .print();
            }
        }
    }
}

trait Visitable {
    fn visit(self, v: &mut Visitor);
}

trait Named<'a> {
    fn name(&'a self) -> &'a str;
    fn loc(&'a self) -> &'a parser::Span;
}

macro_rules! impl_named {
    ($x:ty) => {
        impl<'a> Named<'a> for $x {
            fn name(&'a self) -> &'a str {
                &self.name
            }
            fn loc(&'a self) -> &'a parser::Span {
                &self.loc
            }
        }
    };
}

impl_named!(ast::NamespaceDecl);
impl_named!(ast::StructDecl);
impl_named!(ast::FunctionDecl);
impl_named!(ast::Field);

impl Visitable for &ast::Module {
    fn visit(self, v: &mut Visitor) {
        v.check_dupes("namespace", &self.namespaces);
        for ns in &self.namespaces {
            v.visit(ns);
        }
    }
}

impl Visitable for &ast::NamespaceDecl {
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

impl Visitable for &ast::StructDecl {
    fn visit(self, v: &mut Visitor) {
        v.check_dupes("field", &self.fields);
        for p in &self.fields {
            v.visit(p);
        }
    }
}

impl Visitable for &ast::FunctionDecl {
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

impl Visitable for &ast::Field {
    fn visit(self, _v: &mut Visitor) {}
}

pub fn check(module: &ast::Module) -> Result<(), Error> {
    let mut v = Visitor { num_errors: 0 };
    v.visit(module);
    if v.num_errors > 0 {
        Err(Error {
            num_errors: v.num_errors,
        })
    } else {
        Ok(())
    }
}
