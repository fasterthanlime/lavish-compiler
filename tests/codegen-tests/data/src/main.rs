mod test;

#[cfg(test)]
mod tests {
    use super::test::*;
    use std::sync::Arc;

    #[test]
    fn test() -> Result<(), Box<dyn std::error::Error + 'static>> {
        let addr = {
            let mut r = server::Router::new(Arc::new(()));
            r.handle(get_some_bytes, |_| {
                let value = vec![5, 1, 2, 9];
                Ok(get_some_bytes::Results { value })
            });
            lavish::serve(r, "localhost:0")?.local_addr()
        };

        let r = client::Router::new(Arc::new(()));
        let client = lavish::connect(r, addr)?.client();

        let value = client.call(get_some_bytes::Params {})?.value;
        assert_eq!(value, [5, 1, 2, 9]);

        Ok(())
    }
}
