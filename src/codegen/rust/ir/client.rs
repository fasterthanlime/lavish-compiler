use crate::codegen::rust::prelude::*;

pub struct Client<'a> {
    pub side: ast::Side,
    pub body: ast::Anchored<'a, &'a ast::NamespaceBody>,
}

impl<'a> Client<'a> {
    fn for_each_fun(&self, cb: &mut FnMut(ast::Anchored<&ast::FunctionDecl>)) {
        self.body
            .for_each_fun_of_interface(&mut ast::filter_fun_side(self.side, |f| {
                cb(f);
            }));
    }

    fn define_client(&self, s: &mut Scope) {
        s.write("pub struct Client");
        s.in_block(|s| {
            writeln!(
                s,
                "root: {RootClient},",
                RootClient = self.body.stack.RootClient()
            )
            .unwrap();
        });
        s.lf();

        s.write("impl Client");
        s.in_block(|s| {
            _fn("new")
                .kw_pub()
                .param(format!(
                    "root: {RootClient}",
                    RootClient = self.body.stack.RootClient()
                ))
                .returns("Self")
                .body(|s| {
                    s.write("Self { root }").lf();
                })
                .write_to(s);

            self.for_each_fun(&mut |f| {
                _fn(f.name())
                    .kw_pub()
                    .self_param("&self")
                    .param(format!("p: {Params}", Params = f.Params(&self.body.stack),))
                    .returns(format!(
                        "Result<{Results}, {Error}>",
                        Results = f.Results(&self.body.stack),
                        Error = Structs::Error()
                    ))
                    .body(|s| {
                        s.line("self.root.call(");
                        s.in_scope(|s| {
                            writeln!(
                                s,
                                "{protocol}::Params::{variant}(p),",
                                protocol = self.body.stack.protocol(),
                                variant = f.variant()
                            )
                            .unwrap();
                            s.write("|r| match r");
                            s.in_block(|s| {
                                writeln!(
                                    s,
                                    "{protocol}::Results::{variant}(r) => Some(r),",
                                    protocol = self.body.stack.protocol(),
                                    variant = f.variant(),
                                )
                                .unwrap();
                                writeln!(s, "_ => None,").unwrap();
                            });
                        });
                        s.write(")").lf();
                    })
                    .write_to(s);
            });
        });
    }
}

impl<'a> Display for Client<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        Scope::fmt(f, |s| {
            self.define_client(s);
        })
    }
}
