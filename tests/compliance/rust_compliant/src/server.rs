use super::compliance;
use std::sync::Arc;

pub fn run() {
    let mut r = compliance::server::Router::new(Arc::new(()));
    {
        use compliance::types::*;
        macro_rules! identity_copy {
            ($method: ident) => {
                r.handle($method, |call| Ok($method::Results { x: call.params.x }));
            };
        }
        macro_rules! identity_clone {
            ($method: ident) => {
                r.handle($method, |call| {
                    Ok($method::Results {
                        x: call.params.x.clone(),
                    })
                });
            };
        }

        identity_copy!(identity_u8);
        identity_copy!(identity_u16);
        identity_copy!(identity_u32);
        identity_copy!(identity_u64);

        identity_copy!(identity_i8);
        identity_copy!(identity_i16);
        identity_copy!(identity_i32);
        identity_copy!(identity_i64);

        identity_copy!(identity_bool);

        identity_clone!(identity_string);
        identity_clone!(identity_data);
        identity_clone!(identity_timestamp);
        identity_clone!(identity_array_string);
        identity_clone!(identity_array_option_u32);
        identity_clone!(identity_option_array_u8);
        identity_clone!(identity_map_string_bool);

        identity_clone!(identity_struct);
        identity_clone!(identity_enum);
    }
    let server = ::lavish::serve_once(r, "localhost:0").unwrap();
    println!("{}", server.local_addr());
    server.join().unwrap();
    println!("kk server's done");
}
