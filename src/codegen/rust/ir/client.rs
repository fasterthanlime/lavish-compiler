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
            s.line("// TODO");
        });
        s.lf();

        s.write("impl Client");
        s.in_block(|s| {
            self.for_each_fun(&mut |f| {
                _fn(f.name())
                    .kw_pub()
                    .body(|s| {
                        s.line("// TODO");
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
