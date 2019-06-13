mod services;

#[cfg(test)]
mod tests {
    use super::services::test::*;
    use std::sync::Arc;

    #[test]
    fn test() -> Result<(), Box<dyn std::error::Error + 'static>> {
        let addr = {
            let mut r = server::Router::new(Arc::new(()));
            r.handle(session::login, |call| {
                Ok(session::login::Results {
                    session: session::Session {
                        username: call.params.username.clone(),
                        display_name: "John Doe".into(),
                    },
                })
            });
            lavish::serve(r, "localhost:0")?.local_addr()
        };

        let r = client::Router::new(Arc::new(()));
        let client = lavish::connect(r, addr)?.client();
        let session = client
            .call(session::login::Params {
                username: "john".into(),
                password: "hunter2".into(),
            })?
            .session;
        assert_eq!(session.display_name, "John Doe");

        Ok(())
    }
}
