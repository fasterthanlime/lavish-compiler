use crate::codegen::rust::prelude::*;

pub struct Router<'a> {
    pub side: ast::Side,
    pub body: ast::Anchored<'a, &'a ast::NamespaceBody>,
}

impl<'a> Router<'a> {
    #[allow(non_snake_case)]
    fn Client(&self) -> String {
        self.body.stack.SideClient(self.side)
    }

    fn for_each_fun(&self, cb: &mut FnMut(ast::Anchored<&ast::FunctionDecl>)) {
        self.body
            .for_each_fun_of_interface(&mut ast::filter_fun_side(self.side, |f| {
                cb(f);
            }));
    }

    fn define_call(&self, s: &mut Scope) {
        s.write("pub struct Call<T, P>");
        s.in_block(|s| {
            writeln!(s, "pub state: {Arc}<T>,", Arc = Structs::Arc()).unwrap();
            writeln!(s, "pub client: {Client},", Client = self.Client()).unwrap();
            s.line("pub params: P,");
        });
        s.lf();

        _impl("Call")
            .type_param("T", None)
            .type_param("P", None)
            .body(|s| {
                _fn("downcast")
                    .self_param("self")
                    .type_param("PP", None)
                    .type_param("F", Some(format!("Fn(P) -> Option<PP>")))
                    .param("f: F")
                    .returns(format!(
                        "Result<Call<T, PP>, {Error}>",
                        Error = Structs::Error()
                    ))
                    .body(|s| {
                        write!(s, "Ok(Call").unwrap();
                        s.in_terminated_block(")", |s| {
                            writeln!(s, "state: self.state,").unwrap();
                            writeln!(s, "client: self.client,").unwrap();
                            writeln!(
                                s,
                                "params: f(self.params).ok_or_else(|| {Error}::WrongParams)?,",
                                Error = Structs::Error()
                            )
                            .unwrap();
                        });
                    })
                    .write_to(s);

                _fn("shutdown_runtime")
                    .self_param("&self")
                    .kw_pub()
                    .body(|s| {
                        s.line("self.client.caller.shutdown_runtime();");
                    })
                    .write_to(s);
            })
            .write_to(s);
    }

    fn define_runtime(&self, s: &mut Scope) {
        writeln!(
            s,
            "pub type Runtime = {lavish}::Runtime<Client>;",
            lavish = Mods::lavish(),
        )
        .unwrap();
    }

    fn define_slot(&self, s: &mut Scope) {
        s.write(format!(
            "pub type SlotReturn = Result<{protocol}::Results, {Error}>;",
            protocol = self.body.stack.protocol(),
            Error = Structs::Error()
        ))
        .lf();

        writeln!(s, "pub type SlotFn<T> = Fn(Call<T, {protocol}::Params>) -> SlotReturn + 'static + Send + Sync;",
            protocol = self.body.stack.protocol(),
        ).unwrap();

        s.write("pub type Slot<T> = Option<Box<SlotFn<T>>>;").lf();
    }

    fn define_router(&self, s: &mut Scope) {
        let t_bound = "Send + Sync + 'static";

        s.write("pub struct Router<T>").lf();
        s.write("where").lf();
        s.scope().write("T: ").write(t_bound).lf();
        s.in_block(|s| {
            s.line("state: std::sync::Arc<T>,");
            writeln!(
                s,
                "slots: {HashMap}<&'static str, Box<SlotFn<T>>>,",
                HashMap = Structs::HashMap(),
            )
            .unwrap();
            self.for_each_fun(&mut |f| {
                writeln!(s, "{slot}: Slot<T>,", slot = f.slot()).unwrap();
            });
        });
        s.lf();

        _impl("Router")
            .type_param("T", Some(t_bound))
            .body(|s| {
                self.write_constructor(s);
                self.write_handle(s);
                self.write_setters(s);
            })
            .write_to(s);

        _impl_trait(
            format!(
                "{lavish}::Handler<Client, {triplet}>",
                lavish = Mods::lavish(),
                triplet = self.body.stack.triplet()
            ),
            "Router",
        )
        .type_param("T", Some(t_bound))
        .body(|s| {
            _fn("handle")
                .self_param("&self")
                .param(format!(
                    "caller: {Caller}, params: {P}",
                    Caller = self.body.stack.Caller(),
                    P = self.body.stack.Params(),
                ))
                .returns(format!(
                    "Result<{R}, {Error}>",
                    R = self.body.stack.Results(),
                    Error = Structs::Error()
                ))
                .body(|s| {
                    self.write_handle_body(s);
                })
                .write_to(s);

            _fn("make_client")
                .param(format!(
                    "caller: {Caller}",
                    Caller = self.body.stack.Caller()
                ))
                .returns("Client")
                .body(|s| {
                    s.line("Client { caller }");
                })
                .write_to(s);
        })
        .write_to(s);
    }

    fn has_variants(&self) -> bool {
        let mut has_variants = false;
        self.for_each_fun(&mut |_| has_variants = true);
        has_variants
    }

    fn write_handle_body(&self, s: &mut Scope) {
        writeln!(s, "use {Atom};", Atom = Traits::Atom()).unwrap();
        if self.has_variants() {
            writeln!(s, "let slot = self.slots.get(params.method())").unwrap();
            s.in_scope(|s| {
                writeln!(
                    s,
                    ".ok_or_else(|| {Error}::MethodUnimplemented(params.method()))?;",
                    Error = Structs::Error(),
                )
                .unwrap();
            });
            s.write("let call = Call");
            s.in_terminated_block(";", |s| {
                writeln!(s, "state: self.state.clone(),").unwrap();
                writeln!(s, "client: {Client} {{ caller }},", Client = self.Client()).unwrap();
                writeln!(s, "params,").unwrap();
            });
            s.write("slot(call)").lf();
        } else {
            writeln!(
                s,
                "Err({Error}::MethodUnimplemented(params.method()))",
                Error = Structs::Error()
            )
            .unwrap();
        }
    }

    fn write_constructor(&self, s: &mut Scope) {
        _fn("new")
            .kw_pub()
            .param(format!("state: {Arc}<T>", Arc = Structs::Arc()))
            .returns("Self")
            .body(|s| {
                s.write("Self");
                s.in_block(|s| {
                    s.line("state,");
                    writeln!(s, "slots: {HashMap}::new(),", HashMap = Structs::HashMap()).unwrap();
                    self.for_each_fun(&mut |f| {
                        s.write(f.slot()).write(": None,").lf();
                    });
                });
            })
            .write_to(s);
    }

    fn write_handle(&self, s: &mut Scope) {
        let stack = &self.body.stack;

        _fn("handle")
            .kw_pub()
            .type_param(
                "S",
                Some(format!(
                    "Fn() -> {Slottable}<P, R>",
                    Slottable = stack.Slottable()
                )),
            )
            .type_param("P", None)
            .type_param(
                "R",
                Some(format!(
                    "{Implementable}<P>",
                    Implementable = stack.Implementable()
                )),
            )
            .type_param(
                "F",
                Some(format!(
                    "Fn(Call<T, P>) -> Result<R, {Error}> + 'static + Send + Sync",
                    Error = Structs::Error()
                )),
            )
            .self_param("&mut self")
            .param("s: S")
            .param("f: F")
            .body(|s| {
                s.write("self.slots.insert(R::method(), Box::new(move |call|");
                s.in_terminated_block("));", |s| {
                    s.line("let call = call.downcast(R::downcast_params)?;");
                    s.line("f(call).map(|r| r.upcast_results())");
                });
            })
            .write_to(s);
    }

    fn write_setters(&self, s: &mut Scope) {
        self.for_each_fun(&mut |f| {
            _fn(f.slot())
                .kw_pub()
                .type_param(
                    "F",
                    Some(format!(
                        "Fn(Call<T, {PP}>) -> Result<{RR}, {Error}> + Send + Sync + 'static",
                        PP = f.Params(&self.body.stack),
                        RR = f.Results(&self.body.stack),
                        Error = Structs::Error(),
                    )),
                )
                .self_param("&mut self")
                .param("f: F")
                .body(|s| {
                    self.write_setter_body(s, &f);
                })
                .write_to(s);
        });
    }

    fn write_setter_body(&self, s: &mut Scope, f: &ast::Anchored<&ast::FunctionDecl>) {
        writeln!(s, "self.{slot} = Some(Box::new(", slot = f.slot()).unwrap();
        s.in_scope(|s| {
            write!(s, "move |call|").unwrap();
            s.in_block(|s| {
                write!(s, "let call = call.downcast(|p| match p").unwrap();
                s.in_terminated_block(")?;", |s| {
                    writeln!(
                        s,
                        "{protocol}::Params::{variant}(p) => Some(p),",
                        protocol = self.body.stack.protocol(),
                        variant = f.variant()
                    )
                    .unwrap();
                    writeln!(s, "_ => None,").unwrap();
                });
                writeln!(
                    s,
                    "f(call).map({protocol}::Results::{variant})",
                    protocol = self.body.stack.protocol(),
                    variant = f.variant()
                )
                .unwrap();
            });
        });
        writeln!(s, "));").unwrap();
    }
}

impl<'a> Display for Router<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        Scope::fmt(f, |s| {
            self.define_runtime(s);
            self.define_call(s);
            self.define_slot(s);
            self.define_router(s);
        })
    }
}
