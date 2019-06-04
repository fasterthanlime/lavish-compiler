use crate::codegen::rust::prelude::*;

pub struct Handler<'a> {
    pub side: ast::Side,
    pub body: ast::Anchored<'a, &'a ast::NamespaceBody>,
}

impl<'a> Handler<'a> {
    fn for_each_fun(&self, cb: &mut FnMut(ast::Anchored<&ast::FunctionDecl>)) {
        self.body
            .for_each_fun_of_interface(&mut ast::filter_fun_side(self.side, |f| {
                cb(f);
            }));
    }

    fn define_call(&self, s: &mut Scope) {
        s.write("pub struct Call<T, PP>");
        s.in_block(|s| {
            s.line(format!("pub state: {Arc}<T>,", Arc = Structs::Arc()));
            s.line("pub client: Client,");
            s.line("pub params: PP,");
        });
        s.lf();
    }

    fn define_slot(&self, s: &mut Scope) {
        s.write(format!(
            "pub type SlotReturn = Result<{protocol}::Results, {Error}>;",
            protocol = self.body.stack.protocol(),
            Error = Structs::Error()
        ))
        .lf();

        writeln!(s, "pub type SlotFn<T> = Fn({Arc}<T>, Client, {protocol}::Params) -> SlotReturn + 'static + Send + Sync;",
            Arc = Structs::Arc(),
            protocol = self.body.stack.protocol(),
        ).unwrap();

        s.write("pub type Slot<T> = Option<Box<SlotFn<T>>>;").lf();
    }

    fn define_handler(&self, s: &mut Scope) {
        s.write("pub struct Handler<T>");
        s.in_block(|s| {
            s.line("state: std::sync::Arc<T>,");
            self.for_each_fun(&mut |f| {
                writeln!(s, "{slot}: Slot<T>,", slot = f.slot()).unwrap();
            });
        });
        s.lf();

        _impl("Handler")
            .type_param("T", None)
            .body(|s| {
                self.write_constructor(s);
                self.write_setters(s);
            })
            .write_to(s);

        _impl_trait(
            format!(
                "{lavish}::Handler<{triplet}>",
                lavish = Mods::lavish(),
                triplet = self.body.stack.triplet()
            ),
            "Handler",
        )
        .type_param("T", Some("Send + Sync"))
        .body(|s| {
            _fn("handle")
                .self_param("&self")
                .param(format!(
                    "root: {RootClient}, params: {P}",
                    RootClient = self.body.stack.RootClient(),
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
        })
        .write_to(s);
    }

    fn write_handle_body(&self, s: &mut Scope) {
        writeln!(
            s,
            "let call = Call {{ state: self.state.clone(), client: {Client} {{ root }}, params }};",
            Client = format!(
                "super::{other_side}::Client",
                other_side = self.side.other()
            ),
        )
        .unwrap();
        s.write("match params");
        let match_end = format!(
            ".unwrap_or_else(|| {Error}::UnimplementedMethod)(call)",
            Error = Structs::Error(),
        );
        s.in_terminated_block(match_end, |s| {
            self.for_each_fun(&mut |f| {
                writeln!(
                    s,
                    "{Params}::{variant}(p) => self.{slot},",
                    Params = self.body.stack.Params(),
                    variant = f.variant(),
                    slot = f.slot(),
                )
                .unwrap();
            });
            writeln!(s, "_ => None,").unwrap();
        });
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
                    self.for_each_fun(&mut |f| {
                        s.write(f.slot()).write(": None,").lf();
                    });
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
            write!(s, "|state, client, params|").unwrap();
            s.in_block(|s| {
                write!(s, "let params = match params").unwrap();
                s.in_terminated_block(";", |s| {
                    writeln!(
                        s,
                        "{protocol}::Params::{variant}(p) => p,",
                        protocol = self.body.stack.protocol(),
                        variant = f.variant()
                    )
                    .unwrap();
                    writeln!(
                        s,
                        "_ => return Err({Error}::WrongParams),",
                        Error = Structs::Error()
                    )
                    .unwrap();
                });
                writeln!(s, "let call = Call {{ state, client, params }};").unwrap();
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

impl<'a> Display for Handler<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        Scope::fmt(f, |s| {
            self.define_call(s);
            self.define_slot(s);
            self.define_handler(s);
        })
    }
}
