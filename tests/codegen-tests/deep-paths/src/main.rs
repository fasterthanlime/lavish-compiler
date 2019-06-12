mod services;

#[cfg(test)]
mod tests {
    use super::services::test::*;
    use std::sync::Arc;

    #[test]
    fn test() -> Result<(), Box<dyn std::error::Error + 'static>> {
        let addr = {
            let mut h = server::Handler::new(Arc::new(()));
            h.on_get_baz(|_call| {
                Ok(get_baz::Results {
                    baz: foo::bar::baz::Baz { x: 67 },
                })
            });
            lavish::serve(h, "localhost:0")?.local_addr()
        };

        let h = client::Handler::new(Arc::new(()));
        let client = lavish::connect(h, addr)?.client();
        let res = client.get_baz(get_baz::Params {})?.baz;
        assert_eq!(res.x, 67);

        Ok(())
    }
}
