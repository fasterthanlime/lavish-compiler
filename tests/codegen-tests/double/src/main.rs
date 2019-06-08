mod services;

#[cfg(test)]
mod tests {
    use super::services::test::*;
    use std::sync::Arc;

    #[test]
    fn test() -> Result<(), Box<dyn std::error::Error + 'static>> {
        let addr = {
            let mut h = server::Handler::new(Arc::new(()));
            h.on_double(|call| {
                Ok(double::Results {
                    value: call.params.value * 2,
                })
            });
            lavish::serve(h, "localhost:0")?.local_addr()
        };

        let h = client::Handler::new(Arc::new(()));
        let client = lavish::connect(h, addr)?.client();
        let value = client.double(double::Params { value: 16 })?.value;
        assert_eq!(value, 32);

        Ok(())
    }
}
