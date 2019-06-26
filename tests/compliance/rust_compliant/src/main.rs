mod compliance;
use std::sync::Arc;

fn main() {
    let mut args: Vec<String> = std::env::args().collect();
    args.remove(0);

    match args.get(0) {
        Some(s) => match s.as_ref() {
            "client" => match args.get(1) {
                Some(s) => {
                    client(&s);
                }
                None => usage("missing client address"),
            },
            "server" => {
                server();
            }
            _ => usage("invalid command"),
        },
        _ => usage("no command"),
    };
}

fn usage(msg: &str) {
    println!("Error: {}", msg);
    println!("Usage:");
    println!("   rust_compliant server");
    println!("   rust_compliant client ADDRESS");
    std::process::exit(1);
}

fn eq<T>(expected: T, actual: T)
where
    T: PartialEq<T> + std::fmt::Debug,
{
    if expected != actual {
        panic!("Expected {:?}, got {:?}", expected, actual)
    }
}

fn client(addr: &str) {
    let r = compliance::client::Router::new(Arc::new(()));
    println!("connecting to {:?}", addr);
    let client = ::lavish::connect(r, addr).unwrap().client();

    {
        use compliance::types::*;
        eq(0, client.call(identity_u8::Params { x: 0 }).unwrap().x);
    }
}

fn server() {
    let mut r = compliance::server::Router::new(Arc::new(()));
    {
        use compliance::types::*;
        r.handle(identity_u8, |call| {
            Ok(identity_u8::Results { x: call.params.x })
        });
    }
    let server = ::lavish::serve_once(r, "localhost:0").unwrap();
    println!("{}", server.local_addr());
    server.join().unwrap();
    println!("kk server's done");
}

