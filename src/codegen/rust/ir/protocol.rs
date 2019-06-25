use crate::codegen::rust::prelude::*;

pub struct Protocol<'a> {
    pub body: ast::Anchored<'a, &'a ast::NamespaceBody>,
}

impl<'a> Display for Protocol<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        Scope::fmt(f, |s| {
            s.write("pub mod protocol");
            s.in_block(|s| {
                self.write_atoms(s);
                self.write_specializations(s);
                self.write_translation_tables(s);
            });
        })
    }
}

impl<'a> Protocol<'a> {
    fn write_translation_tables(&self, s: &mut Scope) {
        writeln!(
            s,
            "use {facts}::{{OffsetList, TypeMapping}};",
            facts = Mods::facts()
        )
        .unwrap();

        s.write(derive().debug());
        s.write("pub struct ProtocolMapping").in_block(|s| {
            s.line("// builtins");
            for builtin in get_builtins() {
                writeln!(s, "pub __{variant}: TypeMapping,", variant = builtin.1).unwrap();
            }

            s.line("// structs");
            self.body.for_each_struct_of_schema(&mut |st| {
                writeln!(s, "pub {variant}: TypeMapping,", variant = st.variant()).unwrap();
            });

            s.line("// enums");
            self.body.for_each_enum_of_schema(&mut |en| {
                writeln!(s, "pub {variant}: TypeMapping,", variant = en.variant()).unwrap();
            });
        });
        s.lf();

        s.write("impl Default for ProtocolMapping").in_block(|s| {
            _fn("default")
                .returns("Self")
                .body(|s| {
                    s.write("Self").in_block(|s| {
                        s.line("// builtins");
                        for builtin in get_builtins() {
                            let mut values: Vec<String> = Vec::new();
                            let mut i = 0;
                            self.body.for_each_fun_of_schema(&mut |f| {
                                if f.kind == builtin.0 {
                                    values.push(format!("{}", i));
                                    i += 1;
                                }
                            });

                            writeln!(
                                s,
                                "__{variant}: TypeMapping::Mapped(OffsetList(vec![{values}])),",
                                variant = builtin.1,
                                values = values.join(", "),
                            )
                            .unwrap();
                        }

                        s.line("// structs");
                        self.body.for_each_struct_of_schema(&mut |st| {
                            let mut values: Vec<String> = Vec::new();
                            for i in 0..st.fields.len() {
                                values.push(format!("{}", i));
                            }

                            writeln!(
                                s,
                                "{variant}: TypeMapping::Mapped(OffsetList(vec![{values}])),",
                                variant = st.variant(),
                                values = values.join(", "),
                            )
                            .unwrap();
                        });

                        s.line("// enums");
                        self.body.for_each_enum_of_schema(&mut |en| {
                            let mut values: Vec<String> = Vec::new();
                            for i in 0..en.variants.len() {
                                values.push(format!("{}", i));
                            }

                            writeln!(
                                s,
                                "{variant}: TypeMapping::Mapped(OffsetList(vec![{values}])),",
                                variant = en.variant(),
                                values = values.join(", "),
                            )
                            .unwrap();
                        })
                    });
                })
                .write_to(s);
        });

        writeln!(
            s,
            "impl {facts}::Mapping for ProtocolMapping {{}}",
            facts = Mods::facts()
        )
        .unwrap();
    }

    fn write_atoms(&self, s: &mut Scope) {
        for a in &[
            Atom {
                proto: &self,
                kind: ast::Kind::Request,
                name: "Params",
            },
            Atom {
                proto: &self,
                kind: ast::Kind::Request,
                name: "Results",
            },
            Atom {
                proto: &self,
                kind: ast::Kind::Notification,
                name: "NotificationParams",
            },
        ] {
            s.write(a).lf();
        }
    }

    fn write_specializations(&self, s: &mut Scope) {
        writeln!(
            s,
            "pub type {name} = {lavish}::{name}<{quadruplet}>;",
            lavish = Mods::lavish(),
            quadruplet = self.body.stack.quadruplet(),
            name = "Caller",
        )
        .unwrap();

        writeln!(
            s,
            "pub type {name}<CL> = {lavish}::{name}<CL, {quadruplet}>;",
            lavish = Mods::lavish(),
            quadruplet = self.body.stack.quadruplet(),
            name = "Handler",
        )
        .unwrap();

        s.line("pub trait Callable<R>");
        s.in_block(|s| {
            s.line("fn upcast_params(self) -> Params;");
            s.line("fn downcast_results(results: Results) -> Option<R>;");
        });
        s.lf();

        s.line("pub trait Implementable<P>");
        s.in_block(|s| {
            s.line("fn method() -> &'static str;");
            s.line("fn downcast_params(params: Params) -> Option<P>;");
            s.line("fn upcast_results(self) -> Results;");
        });
        s.lf();

        s.line("#[derive(Clone, Copy)]");
        s.line("pub struct Slottable<P, R>");
        s.line("where");
        s.in_scope(|s| {
            s.line("R: Implementable<P>,");
        });
        s.in_block(|s| {
            s.line("pub phantom: std::marker::PhantomData<(P, R)>,");
        });
        s.lf();
    }
}

pub struct Atom<'a> {
    pub proto: &'a Protocol<'a>,
    pub name: &'a str,
    pub kind: ast::Kind,
}

impl<'a> Atom<'a> {
    fn for_each_fun(&self, cb: &mut FnMut(ast::Anchored<&ast::FunctionDecl>)) {
        let kind = self.kind;
        self.proto.body.for_each_fun_of_schema(&mut |f| {
            if f.kind == kind {
                cb(f);
            }
        });
    }

    fn fun_count(&self) -> usize {
        let mut count = 0;
        self.for_each_fun(&mut |_| count += 1);
        count
    }

    fn implement_method(&self, s: &mut Scope) {
        _fn("method")
            .self_param("&self")
            .returns("&'static str")
            .body(|s| {
                if self.fun_count() == 0 {
                    writeln!(s, "panic!(\"no variants for {}\")", self.name).unwrap();
                    return;
                }

                s.write("match self");
                s.in_block(|s| {
                    self.for_each_fun(&mut |f| {
                        writeln!(
                            s,
                            "{name}::{variant}(_) => {lit},",
                            name = &self.name,
                            variant = f.variant(),
                            lit = quoted(f.method())
                        )
                        .unwrap();
                    });
                });
            })
            .write_to(s);
    }

    fn implement_factual(&self, s: &mut Scope) {
        let stack = &self.proto.body.stack;

        _impl_trait(
            format!(
                "{Factual}<{M}>",
                Factual = Traits::Factual(),
                M = stack.ProtocolMapping()
            ),
            self.name,
        )
        .body(|s| {
            _fn("read")
                .self_bound("Sized")
                .type_param_bound("R", Traits::Read())
                .param(format!(
                    "rd: &mut {Reader}<R>",
                    Reader = Structs::FactsReader()
                ))
                .returns(format!(
                    "Result<Self, {Error}>",
                    Error = Structs::FactsError()
                ))
                .body(|s| {
                    writeln!(s, "let len = rd.read_array_len()?;").unwrap();
                    write!(s, "if len != 2").unwrap();
                    s.in_block(|s| {
                        // FIXME: this might not be ideal, this probably
                        // should be a proper facts error
                        writeln!(s, "unreachable!();").unwrap();
                    });

                    writeln!(s, "let typ: u32 = rd.read_int()?;").unwrap();

                    s.write("match typ");
                    s.in_block(|s| {
                        let mut i = 0;
                        self.for_each_fun(&mut |f| {
                            writeln!(
                                s,
                                "{i} => Ok({name}::{variant}(Self::subread(rd)?)),",
                                i = i,
                                name = &self.name,
                                variant = f.variant(),
                            )
                            .unwrap();

                            i += 1;
                        });
                        // FIXME: proper error
                        s.write("_ => unreachable!(),").lf();
                    });
                })
                .write_to(s);

            s.lf();
            _fn("write")
                .type_param_bound("W", Traits::Write())
                .self_param("&self")
                .param(format!("mapping: &{M}", M = stack.ProtocolMapping()))
                .param("wr: &mut W")
                .returns(format!(
                    "Result<(), {Error}>",
                    Error = Structs::FactsError()
                ))
                .body(|s| {
                    writeln!(s, "let o = &mapping.__{slot};", slot = self.name).unwrap();
                    write!(s, "match self").unwrap();
                    s.in_block(|s| {
                        let mut i = 0;
                        self.for_each_fun(&mut |f| {
                            writeln!(
                                s,
                                "{name}::{variant}(value) =>\n    o.write_union(wr, mapping, {name:?}, {variant:?}, {index}, value),",
                                index = i,
                                name = &self.name,
                                variant = f.variant(),
                            )
                            .unwrap();

                            i += 1;
                        });
                        writeln!(s, "_ => unreachable!(),").unwrap();
                    });
                })
                .write_to(s);
        })
        .write_to(s);
    }
}

impl<'a> Display for Atom<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        Scope::fmt(f, |s| {
            s.write(derive().clone().debug());
            s.write(allow().non_camel_case().unused());
            let mut e = _enum(self.name);
            e.kw_pub();
            self.for_each_fun(&mut |f| {
                e.variant(format!(
                    "{variant}({module}::{name})",
                    variant = f.variant(),
                    module = f.module(&self.proto.body.stack),
                    name = &self.name
                ));
            });
            e.write_to(s);

            _impl_trait(
                format!("{Atom}<ProtocolMapping>", Atom = Traits::Atom()),
                self.name,
            )
            .body(|s| {
                self.implement_method(s);
            })
            .write_to(s);

            self.implement_factual(s);
        })
    }
}

fn get_builtins() -> Vec<(ast::Kind, String)> {
    vec![
        (ast::Kind::Request, "Params".into()),
        (ast::Kind::Request, "Results".into()),
        (ast::Kind::Notification, "NotificationParams".into()),
    ]
}
