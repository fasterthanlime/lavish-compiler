mod services;

#[cfg(test)]
mod tests {
    use super::services::double;
    use std::net::{TcpListener, TcpStream};
    use std::sync::Arc;

    #[test]
    fn test() {
        let addr = "127.0.0.1:9095";
        let listener = TcpListener::bind(addr).unwrap();

        let server = std::thread::spawn(move || {
            let conn = listener.incoming().next().unwrap().unwrap();

            let mut h = double::server::Handler::new(Arc::new(()));
            h.on_double(|call| {
                Ok(double::double::Results {
                    value: call.params.value * 2,
                })
            });
            let (mut runtime, _client) = h.spawn(conn).unwrap();
            runtime.join().unwrap();
        });

        let client = std::thread::spawn(move || {
            let conn = TcpStream::connect(addr).unwrap();
            let h = double::client::Handler::new(Arc::new(()));
            let (_runtime, client) = h.spawn(conn).unwrap();

            let value = client
                .double(double::double::Params { value: 16 })
                .unwrap()
                .value;
            assert_eq!(value, 32);
        });

        server.join().unwrap();
        client.join().unwrap();
    }
}
