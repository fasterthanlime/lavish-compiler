mod test;

#[cfg(test)]
mod tests {
    use super::test::*;
    use std::sync::Arc;

    #[test]
    fn test() -> Result<(), Box<dyn std::error::Error + 'static>> {
        let addr = {
            let mut r = server::Router::new(Arc::new(()));
            r.handle(get_baz, |_call| {
                Ok(get_baz::Results {
                    baz: foo::bar::baz::Baz { x: 67 },
                })
            });
            lavish::serve(r, "localhost:0")?.local_addr()
        };

        let r = client::Router::new(Arc::new(()));
        let client = lavish::connect(r, addr)?.client();
        let res = client.call(get_baz::Params {})?.baz;
        assert_eq!(res.x, 67);

        Ok(())
    }
}
