mod services;

#[cfg(test)]
mod tests {
    use super::services::test::*;
    use std::sync::Arc;

    #[test]
    fn test() -> Result<(), Box<dyn std::error::Error + 'static>> {
        let addr = {
            let mut h = server::Handler::new(Arc::new(()));
            h.on_session__login(|call| {
                Ok(session::login::Results {
                    session: session::Session {
                        username: call.params.username.clone(),
                        display_name: "John Doe".into(),
                    },
                })
            });
            lavish::serve(h, "localhost:0")?.local_addr()
        };

        let h = client::Handler::new(Arc::new(()));
        let client = lavish::connect(h, addr)?.client();
        let session = client
            .session__login(session::login::Params {
                username: "john".into(),
                password: "hunter2".into(),
            })?
            .session;
        assert_eq!(session.display_name, "John Doe");

        Ok(())
    }
}