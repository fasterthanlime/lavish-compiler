use crate::codegen::rust::prelude::*;

pub struct Symbols<'a> {
    body: ast::Anchored<'a, &'a ast::NamespaceBody>,
}

impl<'a> Symbols<'a> {
    pub fn new(body: ast::Anchored<'a, &'a ast::NamespaceBody>) -> Self {
        Self { body }
    }
}

impl<'a> Display for Symbols<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        Scope::fmt(f, |s| {
            let body = &self.body;
            let stack = &body.stack;

            for node in &body.structs {
                s.write(Struct::new(stack.anchor(node)));
            }
            for node in &body.functions {
                s.write(Function::new(stack.anchor(node)));
            }

            for ns in &body.inner.namespaces {
                write!(s, "pub mod {}", ns.name.text).unwrap();
                s.in_block(|s| {
                    s.write(Symbols::new(stack.push(ns).anchor(&ns.body)));
                });
            }
        })
    }
}

pub struct Struct<'a> {
    node: ast::Anchored<'a, &'a ast::StructDecl>,
}

impl<'a> Struct<'a> {
    fn new(node: ast::Anchored<'a, &'a ast::StructDecl>) -> Self {
        Self { node }
    }
}

impl<'a> Display for Struct<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        Scope::fmt(f, |s| {
            s.write(derive().debug().serialize().deserialize());
            s.write("pub struct ").write(self.node.name());
            s.in_block(|s| {
                for f in &self.node.fields {
                    s.write(Field::new(self.node.stack.anchor(f)))
                        .write(",")
                        .lf();
                }
            });
        })
    }
}

pub struct Function<'a> {
    node: ast::Anchored<'a, &'a ast::FunctionDecl>,
}

impl<'a> Function<'a> {
    fn new(node: ast::Anchored<'a, &'a ast::FunctionDecl>) -> Self {
        Self { node }
    }
}

impl<'a> Display for Function<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        Scope::fmt(f, |s| {
            s.write("pub mod ").write(self.node.name());
            s.in_block(|s| {
                s.write(derive().debug().serialize().deserialize());
                s.write("pub struct Params");
                s.in_block(|s| {
                    for f in &self.node.params {
                        s.write(Field::new(self.node.stack.anchor(f)))
                            .write(",")
                            .lf();
                    }
                });

                s.lf();

                s.write(derive().debug().serialize().deserialize());
                s.write("pub struct Results");
                s.in_block(|s| {
                    for f in &self.node.results {
                        s.write(Field::new(self.node.stack.anchor(f)))
                            .write(",")
                            .lf();
                    }
                });

                if let Some(body) = self.node.body.as_ref() {
                    s.lf();
                    let stack = self.node.stack.push(self.node.inner);

                    for node in &body.functions {
                        s.write(Function::new(stack.anchor(node)));
                    }

                    s.write(super::client::Client {
                        side: self.node.side,
                        body: stack.anchor(body),
                    });
                }
            });
        })
    }
}

pub struct Field<'a> {
    node: ast::Anchored<'a, &'a ast::Field>,
}
impl<'a> Field<'a> {
    fn new(node: ast::Anchored<'a, &'a ast::Field>) -> Self {
        Self { node }
    }
}

impl<'a> Display for Field<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        Scope::fmt(f, |s| {
            write!(
                s,
                "{name}: {typ}",
                name = self.node.name(),
                typ = self.node.typ.as_rust(&self.node.stack)
            )
            .unwrap();
        })
    }
}
