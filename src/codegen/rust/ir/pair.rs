use crate::codegen::rust::prelude::*;

use super::client::Client;
use super::router::Router;

pub fn write_pair(s: &mut Scope, body: ast::Anchored<&ast::NamespaceBody>) {
    {
        write!(s, "pub mod client").unwrap();
        s.in_block(|s| {
            let frame = ast::SyntheticFrame::new("client");
            let stack = body.stack.push(&frame);

            s.write(Client {
                body: stack.anchor(body.inner),
                side: ast::Side::Server,
            });

            s.write(Router {
                body: stack.anchor(body.inner),
                side: ast::Side::Client,
            });
        });
        s.lf();
    }

    {
        write!(s, "pub mod server").unwrap();
        s.in_block(|s| {
            let frame = ast::SyntheticFrame::new("server");
            let stack = body.stack.push(&frame);

            s.write(Client {
                body: stack.anchor(body.inner),
                side: ast::Side::Client,
            });

            s.write(Router {
                body: stack.anchor(body.inner),
                side: ast::Side::Server,
            });
        });
        s.lf();
    }
}
