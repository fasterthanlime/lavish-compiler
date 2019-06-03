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
            });
        })
    }
}

impl<'a> Protocol<'a> {
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
        let triplet = {
            let protocol = self.body.stack.protocol();
            format!(
                "{P}, {NP}, {R}",
                P = format!("{}::Params", protocol),
                NP = format!("{}::NotificationParams", protocol),
                R = format!("{}::Results", protocol)
            )
        };

        let mut specialize = |name: &str| {
            writeln!(
                s,
                "pub type {name} = {lavish}::{name}<{triplet}>;",
                lavish = Mods::lavish(),
                triplet = triplet,
                name = name,
            )
            .unwrap()
        };

        specialize("Client");
        specialize("Handler");
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

    fn implement_deserialize(&self, s: &mut Scope) {
        _fn("deserialize")
            .param("method: &str")
            .param(format!(
                "de: &mut {Deserializer}",
                Deserializer = Structs::Deserializer()
            ))
            .returns(format!("{es}::Result<Self>", es = Mods::es()))
            .body(|s| {
                writeln!(s, "use {es}::deserialize as __DS;", es = Mods::es()).unwrap();
                writeln!(s, "use {serde}::de::Error;", serde = Mods::serde()).unwrap();
                s.lf();

                s.write("match method");
                s.in_block(|s| {
                    self.for_each_fun(&mut |f| {
                        s.line(format!("{method} => ", method = quoted(f.method())));
                        s.scope()
                            .write("Ok")
                            .in_list(Brackets::Round, |l| {
                                l.item(format!(
                                    "{name}::{variant}(__DS::<{module}::{name}>(de)?)",
                                    name = &self.name,
                                    variant = f.variant(),
                                    module = f.module(&self.proto.body.stack),
                                ));
                            })
                            .write(",")
                            .lf();
                    });
                    s.write("_ =>").lf();
                    s.scope()
                        .write("Err")
                        .in_parens(|s| {
                            s.write(format!("{es}::Error::custom", es = Mods::es()))
                                .in_parens(|s| {
                                    s.write("format!").in_parens_list(|l| {
                                        l.item(quoted("unknown method: {}"));
                                        l.item("method")
                                    });
                                });
                        })
                        .write(",")
                        .lf();
                });
            })
            .write_to(s);
    }
}

impl<'a> Display for Atom<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        Scope::fmt(f, |s| {
            s.write(derive().debug().serialize());
            s.write(allow().non_camel_case().unused());
            s.write(serde_untagged());
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

            _impl(Traits::Atom(), self.name)
                .body(|s| {
                    self.implement_method(s);
                    self.implement_deserialize(s);
                })
                .write_to(s);
        })
    }
}
