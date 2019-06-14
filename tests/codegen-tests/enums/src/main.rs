mod test;

#[cfg(test)]
mod tests {
    use super::test::*;
    use std::sync::Arc;

    #[test]
    fn test() -> Result<(), Box<dyn std::error::Error + 'static>> {
        let addr = {
            let mut r = server::Router::new(Arc::new(()));
            r.handle(get_flavor_name, |call| {
                Ok(get_flavor_name::Results {
                    name: format!("{:?}", call.params.flavor),
                })
            });
            lavish::serve(r, "localhost:0")?.local_addr()
        };

        let r = client::Router::new(Arc::new(()));
        let client = lavish::connect(r, addr)?.client();

        let name = client
            .call(get_flavor_name::Params {
                flavor: Flavor::Vanilla,
            })?
            .name;
        assert_eq!(name, "Vanilla");
        let name = client
            .call(get_flavor_name::Params {
                flavor: Flavor::Chocolate,
            })?
            .name;
        assert_eq!(name, "Chocolate");

        Ok(())
    }
}
