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
            let stack = &self.node.stack;
            _fn(self.node.name())
                .kw_pub()
                .returns(format!(
                    "{Slottable}<{name}::Params, {name}::Results>",
                    Slottable = stack.Slottable(),
                    name = self.node.name(),
                ))
                .body(|s| {
                    writeln!(
                        s,
                        "{Slottable} {{ phantom: std::marker::PhantomData }}",
                        Slottable = stack.Slottable()
                    )
                    .unwrap();
                })
                .write_to(s);

            s.write("pub mod ").write(self.node.name());
            s.in_block(|s| {
                let stack = stack.push(self.node.inner);
                s.write(derive().debug().serialize().deserialize());
                s.write("pub struct Params");
                s.in_block(|s| {
                    for f in &self.node.params {
                        s.write(Field::new(stack.anchor(f))).write(",").lf();
                    }
                });

                s.lf();

                s.write(derive().debug().serialize().deserialize());
                s.write("pub struct Results");
                s.in_block(|s| {
                    for f in &self.node.results {
                        s.write(Field::new(stack.anchor(f))).write(",").lf();
                    }
                });

                s.lf();

                _impl_trait(
                    format!("{Callable}<Results>", Callable = stack.Callable()),
                    "Params",
                )
                .body(|s| {
                    _fn("upcast_params")
                        .self_param("self")
                        .returns(stack.Params())
                        .body(|s| {
                            writeln!(
                                s,
                                "{Params}::{variant}(self)",
                                Params = stack.Params(),
                                variant = self.node.variant()
                            )
                            .unwrap();
                        })
                        .write_to(s);

                    _fn("downcast_results")
                        .param(format!("results: {Results}", Results = stack.Results()))
                        .returns("Option<Results>")
                        .body(|s| {
                            s.write("match results");
                            s.in_block(|s| {
                                writeln!(
                                    s,
                                    "{Results}::{variant}(r) => Some(r),",
                                    Results = stack.Results(),
                                    variant = self.node.variant()
                                )
                                .unwrap();
                                s.line("_ => None,");
                            });
                        })
                        .write_to(s);
                })
                .write_to(s);

                s.lf();

                _impl_trait(
                    format!(
                        "{Implementable}<Params>",
                        Implementable = stack.Implementable()
                    ),
                    "Results",
                )
                .body(|s| {
                    _fn("method")
                        .returns("&'static str")
                        .body(|s| {
                            writeln!(s, "{:?}", self.node.method()).unwrap();
                        })
                        .write_to(s);
                    _fn("upcast_results")
                        .self_param("self")
                        .returns(stack.Results())
                        .body(|s| {
                            writeln!(
                                s,
                                "{Results}::{variant}(self)",
                                Results = stack.Results(),
                                variant = self.node.variant(),
                            )
                            .unwrap();
                        })
                        .write_to(s);
                    _fn("downcast_params")
                        .param(format!("params: {Params}", Params = stack.Params()))
                        .returns("Option<Params>")
                        .body(|s| {
                            s.write("match params");
                            s.in_block(|s| {
                                writeln!(
                                    s,
                                    "{Params}::{variant}(p) => Some(p),",
                                    Params = stack.Params(),
                                    variant = self.node.variant()
                                )
                                .unwrap();
                                s.line("_ => None,");
                            });
                        })
                        .write_to(s);
                })
                .write_to(s);

                if let Some(body) = self.node.body.as_ref() {
                    s.lf();

                    for node in &body.functions {
                        s.write(Function::new(stack.anchor(node)));
                    }
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
            s.comment(&self.node.comment);
            write!(
                s,
                "pub {name}: {typ}",
                name = self.node.name(),
                typ = self.node.typ.as_rust(&self.node.stack)
            )
            .unwrap();
        })
    }
}
