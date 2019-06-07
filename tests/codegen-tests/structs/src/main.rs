mod services;

#[cfg(test)]
mod tests {
    use super::services::structs;
    use std::sync::Arc;

    #[test]
    fn test() -> Result<(), Box<dyn std::error::Error + 'static>> {
        let addr = {
            let mut h = structs::server::Handler::new(Arc::new(()));
            h.on_login(|call| {
                // TODO: check credentials
                Ok(structs::login::Results {
                    session: structs::Session {
                        username: call.params.username.clone(),
                        display_name: "John Doe".into(),
                    },
                })
            });
            lavish::serve(h, "localhost:0")?.local_addr()
        };

        let h = structs::client::Handler::new(Arc::new(()));
        let client = lavish::connect(h, addr)?.client();
        let session = client
            .login(structs::login::Params {
                username: "john".into(),
                password: "hunter2".into(),
            })?
            .session;
        assert_eq!(session.display_name, "John Doe");

        Ok(())
    }
}
