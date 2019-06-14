mod test;

#[cfg(test)]
mod tests {
    use super::test::*;
    use std::sync::Arc;

    #[test]
    fn test() -> Result<(), Box<dyn std::error::Error + 'static>> {
        let addr = {
            let mut r = server::Router::new(Arc::new(()));
            r.handle(double, |call| {
                Ok(double::Results {
                    value: call.params.value * 2,
                })
            });
            lavish::serve(r, "localhost:0")?.local_addr()
        };

        let r = client::Router::new(Arc::new(()));
        let client = lavish::connect(r, addr)?.client();
        let value = client.call(double::Params { value: 16 })?.value;
        assert_eq!(value, 32);

        Ok(())
    }
}
