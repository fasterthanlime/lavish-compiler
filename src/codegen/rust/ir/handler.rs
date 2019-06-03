use crate::codegen::rust::prelude::*;

pub struct Handler<'a> {
    pub side: ast::Side,
    pub body: ast::Anchored<'a, &'a ast::NamespaceBody>,
}

impl<'a> Client<'a> {
    fn for_each_fun(&self, cb: &mut FnMut(ast::Anchored<&ast::FunctionDecl>)) {
        self.body.walk_client_funs(cb);
    }

    fn for_each_handled_fun(&self, cb: &mut FnMut(ast::Anchored<&ast::FunctionDecl>)) {
        self.for_each_fun(&mut |f| {
            if f.side == self.side {
                cb(f);
            }
        });
    }

    fn for_each_called_fun(&self, cb: &mut FnMut(ast::Anchored<&ast::FunctionDecl>)) {
        self.for_each_fun(&mut |f| {
            if f.side == self.side.other() {
                cb(f);
            }
        });
    }

    fn define_client(&self, s: &mut Scope) {
        s.write("pub struct Client");
        s.in_block(|s| {
            self.for_each_called_fun(&mut |f| {
                writeln!(s, "{qfn}: (),", qfn = f.qualified_name()).unwrap();
            });
            s.line("// TODO");
        });
        s.lf();
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
            self.for_each_handled_fun(&mut |f| {
                writeln!(s, "on_{fqn}: Slot<T>,", fqn = f.qualified_name()).unwrap();
            });
        });
        s.lf();
    }
}

impl<'a> Display for Client<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        Scope::fmt(f, |s| {
            self.define_call(s);
            self.define_slot(s);
            self.define_handler(s);
        })
    }
}
