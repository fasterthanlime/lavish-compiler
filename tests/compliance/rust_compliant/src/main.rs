mod client;
mod compliance;
mod server;

fn main() {
    let mut args: Vec<String> = std::env::args().collect();
    args.remove(0);

    match args.get(0) {
        Some(s) => match s.as_ref() {
            "client" => match args.get(1) {
                Some(s) => {
                    client::run(&s);
                }
                None => usage("missing client address"),
            },
            "server" => {
                server::run();
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
