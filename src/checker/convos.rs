use super::ast;
use super::Error;
use colored::*;

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
}

trait Visitable {
    fn visit(self, v: &mut Visitor);
}

impl Visitable for &ast::Schema {
    fn visit(self, v: &mut Visitor) {
        for ns in &self.body.namespaces {
            v.visit(ns);
        }
    }
}

impl Visitable for &ast::NamespaceDecl {
    fn visit(self, v: &mut Visitor) {
        self.body.visit(v)
    }
}

impl Visitable for &ast::NamespaceBody {
    fn visit(self, v: &mut Visitor) {
        for ns in &self.namespaces {
            v.visit(ns);
        }
        for f in &self.functions {
            v.visit(f);
        }
    }
}

impl Visitable for &ast::FunctionDecl {
    fn visit(self, v: &mut Visitor) {
        if let Some(body) = self.body.as_ref() {
            let expected_side = self.side.other();
            for f in &body.functions {
                if f.side != expected_side {
                    v.num_errors += 1;
                    f.name
                        .span
                        .position()
                        .diag_err(format!(
                            "{} {} should be {}",
                            "error:".red().bold(),
                            f.name.text(),
                            expected_side
                        ))
                        .print();
                    self.name
                        .span
                        .position()
                        .diag_info(format!(
                            "because its parent, {}, is {}",
                            self.name.text(),
                            self.side
                        ))
                        .print();
                }
            }

            v.visit(body);
        }
    }
}

impl Visitable for &ast::Field {
    fn visit(self, _v: &mut Visitor) {}
}

pub fn check(schema: &ast::Schema) -> Result<(), Error> {
    let mut v = Visitor { num_errors: 0 };
    v.visit(schema);
    if v.num_errors > 0 {
        Err(Error {
            num_errors: v.num_errors,
        })
    } else {
        Ok(())
    }
}
