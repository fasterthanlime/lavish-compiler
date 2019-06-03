use super::*;
use crate::ast;
use crate::codegen::output::*;
use std::fmt::{self, Display, Write};

pub struct Protocol<'a> {
    pub body: Anchored<'a, &'a ast::NamespaceBody>,
}

impl<'a> Display for Protocol<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        Scope::fmt(f, |s| {
            s.write("pub mod protocol");
            s.in_block(|s| {
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
            });
        })
    }
}

pub struct Atom<'a> {
    pub proto: &'a Protocol<'a>,
    pub name: &'a str,
    pub kind: ast::Kind,
}

impl<'a> Atom<'a> {
    fn for_each_fun(&self, cb: &mut FnMut(Anchored<&ast::FunctionDecl>)) {
        let kind = self.kind;
        self.proto.body.walk_all_funs(&mut |f| {
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
                                    "{name}::{variant}(__DS::<{schema}::{module}::{name}>(de)?)",
                                    schema = self.proto.body.stack.schema(),
                                    name = &self.name,
                                    variant = f.variant(),
                                    module = f.module(),
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
                    "{variant}({schema}::{module}::{name})",
                    variant = f.variant(),
                    schema = self.proto.body.stack.schema(),
                    module = f.module(),
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
