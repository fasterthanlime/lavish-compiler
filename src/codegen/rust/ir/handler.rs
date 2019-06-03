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

        writeln!(s, 
            "pub type SlotFn<T> = Fn({Arc}<T>, Client, {protocol}::Params) -> SlotReturn + 'static + Send + Sync;",
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
                writeln!(s, "on_{qfn}: Slot<T>,", qfn = f.qualified_name()).unwrap();
            });
        });
        s.lf();

        s.write("impl<T> Handler<T>");
        s.in_block(|s| {
            self.for_each_fun(&mut |f| {
                let qfn = f.qualified_name();

                _fn(format!("on_{qfn}", qfn = qfn)).kw_pub().type_param("F", Some("Fn(Call)")).self_param("&mut self").param("f: F").body(|s| {
                    writeln!(s, "self.on_{qfn} = f.into();", qfn = qfn).unwrap();
                }).write_to(s);
            });
        });
        s.lf();
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
