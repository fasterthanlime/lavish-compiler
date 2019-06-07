mod services;

#[cfg(test)]
mod tests {
    use super::services::double;
    use std::sync::Arc;

    #[test]
    fn test() -> Result<(), Box<dyn std::error::Error + 'static>> {
        let addr = {
            let mut h = double::server::Handler::new(Arc::new(()));
            h.on_double(|call| {
                Ok(double::double::Results {
                    value: call.params.value * 2,
                })
            });
            lavish::serve(h, "localhost:0")?.local_addr()
        };

        let h = double::client::Handler::new(Arc::new(()));
        let client = lavish::connect(h, addr)?.client();
        let value = client.double(double::double::Params { value: 16 })?.value;
        assert_eq!(value, 32);

        Ok(())
    }
}
