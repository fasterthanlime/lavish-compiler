mod test;

#[cfg(test)]
mod tests {
    use super::test::*;
    use std::collections::HashMap;
    use std::sync::Arc;

    #[test]
    fn test() -> Result<(), Box<dyn std::error::Error + 'static>> {
        let addr = {
            let mut r = server::Router::new(Arc::new(()));
            r.handle(get_kitchen_sink, |_| {
                let optional_array =
                    vec![Some("first".to_string()), None, Some("third".to_string())];

                let mut optional_map = HashMap::<i32, Option<Vec<u8>>>::new();
                optional_map.insert(45, None);
                optional_map.insert(91, Some(vec![9, 1]));

                let mut scores = HashMap::<String, i32>::new();
                scores.insert("Giselle".into(), 256);
                scores.insert("Joanna".into(), 314);

                let sink = KitchenSink {
                    numbers: vec![1, 2, 3],
                    scores,
                    set: Some("surprise".into()),
                    unset: None,
                    optional_array,
                    optional_map,
                };
                Ok(get_kitchen_sink::Results { sink })
            });
            lavish::serve(r, "localhost:0")?.local_addr()
        };

        let r = client::Router::new(Arc::new(()));
        let client = lavish::connect(r, addr)?.client();

        let sink = client.call(get_kitchen_sink::Params {})?.sink;

        assert_eq!(sink.numbers, [1, 2, 3]);

        assert_eq!(sink.scores["Giselle"], 256);
        assert_eq!(sink.scores["Joanna"], 314);

        assert!(sink.optional_map.get(&45).unwrap().is_none());
        {
            let val = sink.optional_map.get(&91).unwrap();
            assert!(val.is_some());
            assert_eq!(&val.as_ref().unwrap()[..], [9, 1]);
        }

        assert_eq!(sink.optional_array[0].as_ref().unwrap(), "first");
        assert!(sink.optional_array[1].is_none());
        assert_eq!(sink.optional_array[2].as_ref().unwrap(), "third");

        assert!(sink.unset.is_none());
        assert!(sink.set.is_some());

        Ok(())
    }
}
