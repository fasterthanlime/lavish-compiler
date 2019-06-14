mod test;

#[cfg(test)]
mod tests {
    use super::test::*;
    use lavish::chrono::{offset::Utc, Duration};
    use std::sync::Arc;

    #[test]
    fn test() -> Result<(), Box<dyn std::error::Error + 'static>> {
        let addr = {
            let mut r = server::Router::new(Arc::new(()));
            r.handle(get_timestamp, |_| {
                Ok(get_timestamp::Results { value: Utc::now() })
            });
            r.handle(subtract_week, |call| {
                Ok(subtract_week::Results {
                    value: call.params.value - Duration::weeks(1),
                })
            });
            lavish::serve(r, "localhost:0")?.local_addr()
        };

        let r = client::Router::new(Arc::new(()));
        let client = lavish::connect(r, addr)?.client();

        let timestamp = client.call(get_timestamp::Params {})?.value;
        let theirs = client
            .call(subtract_week::Params { value: timestamp })?
            .value;
        let ours = timestamp - Duration::weeks(1);
        assert_eq!(ours, theirs);

        Ok(())
    }
}
