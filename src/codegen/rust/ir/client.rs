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
        s.line("#[derive(Clone)]");
        s.write("pub struct Client");
        s.in_block(|s| {
            writeln!(s, "caller: {Caller},", Caller = self.body.stack.Caller()).unwrap();
        });
        s.lf();

        s.write("impl Client");
        s.in_block(|s| {
            _fn("new")
                .kw_pub()
                .param(format!(
                    "caller: {Caller}",
                    Caller = self.body.stack.Caller()
                ))
                .returns("Self")
                .body(|s| {
                    s.write("Self { caller }").lf();
                })
                .write_to(s);

            _fn("call")
                .kw_pub()
                .type_param(
                    "P",
                    Some(format!(
                        "{Callable}<R>",
                        Callable = self.body.stack.Callable()
                    )),
                )
                .type_param("R", None)
                .self_param("&self")
                .param("p: P")
                .returns(format!("Result<R, {Error}>", Error = Structs::Error()))
                .body(|s| {
                    s.line("self.caller.call(");
                    s.in_scope(|s| {
                        s.line("p.upcast_params(),");
                        s.line("P::downcast_results,");
                    });
                    s.line(")");
                })
                .write_to(s);

            self.for_each_fun(&mut |f| {
                _fn(f.rust_name())
                    .kw_pub()
                    .self_param("&self")
                    .param(format!("p: {Params}", Params = f.Params(&self.body.stack),))
                    .returns(format!(
                        "Result<{Results}, {Error}>",
                        Results = f.Results(&self.body.stack),
                        Error = Structs::Error()
                    ))
                    .body(|s| {
                        s.line("self.caller.call(");
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
