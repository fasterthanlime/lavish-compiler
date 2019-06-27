use super::compliance;
use std::sync::Arc;

pub fn run(addr: &str) {
    let r = compliance::client::Router::new(Arc::new(()));
    println!("connecting to {:?}", addr);
    let client = ::lavish::connect(r, addr).unwrap().client();

    #[allow(clippy::cast_lossless)]
    {
        use compliance::types::*;
        macro_rules! roundtrip {
            ($method: ident, $val: expr) => {
                eq(
                    stringify!($method),
                    $val,
                    client.call($method::Params { x: $val }).unwrap().x,
                );
            };
        }

        roundtrip!(identity_u8, 0);
        roundtrip!(identity_u8, std::u8::MIN);
        roundtrip!(identity_u8, std::u8::MAX);

        roundtrip!(identity_u16, 0);
        roundtrip!(identity_u16, std::u16::MIN);
        roundtrip!(identity_u16, std::u16::MAX);

        roundtrip!(identity_u32, 0);
        roundtrip!(identity_u32, std::u32::MIN);
        roundtrip!(identity_u32, std::u32::MAX);

        roundtrip!(identity_u64, 0);
        roundtrip!(identity_u64, std::u64::MIN);
        roundtrip!(identity_u64, std::u64::MAX);

        roundtrip!(identity_i8, 0);
        roundtrip!(identity_i8, std::i8::MIN);
        roundtrip!(identity_i8, std::i8::MAX);

        roundtrip!(identity_i16, 0);
        roundtrip!(identity_i16, std::i16::MIN);
        roundtrip!(identity_i16, std::i16::MAX);

        roundtrip!(identity_i32, 0);
        roundtrip!(identity_i32, std::i32::MIN);
        roundtrip!(identity_i32, std::i32::MAX);

        roundtrip!(identity_i64, 0);
        roundtrip!(identity_i64, std::i64::MIN);
        roundtrip!(identity_i64, std::i64::MAX);

        roundtrip!(identity_bool, true);
        roundtrip!(identity_bool, false);

        roundtrip!(identity_string, "".to_string());
        roundtrip!(identity_string, "Short".to_string());
        roundtrip!(identity_string, "Long".to_string().repeat(128));
        roundtrip!(identity_string, "Longer".to_string().repeat(10_000));

        roundtrip!(identity_data, lavish::facts::Bin(vec![]));
        roundtrip!(
            identity_data,
            lavish::facts::Bin(vec![0, 13, 61, 23, 0, 32, 51, 12, 0])
        );

        #[allow(clippy::unreadable_literal)]
        {
            let roundtrip_timestamp = |tv_sec: i64, tv_nsec: u32| {
                use lavish::chrono::{offset, DateTime, NaiveDateTime};
                let dt = NaiveDateTime::from_timestamp(tv_sec, tv_nsec);
                let dt = DateTime::from_utc(dt, offset::Utc);
                roundtrip!(identity_timestamp, dt);
            };

            // epoch
            roundtrip_timestamp(0, 0);
            // time at the writing of this test
            roundtrip_timestamp(1561378047, 0);

            // time at the writing of this test, with some nanoseconds
            roundtrip_timestamp(1561378047, 2398);

            // some time in the future (year 2200), no nanoseconds
            roundtrip_timestamp(7273195896, 0);

            // some time in the future (year 2200), with nanoseconds
            roundtrip_timestamp(7273195896, 23549);

            // time of moon landing (before epoch)
            roundtrip_timestamp(-14182980, 0);

            // some time far into the future (year 2600 - amos's 610th birthday)
            roundtrip_timestamp(19898323200, 0);

            // some time far into the future, with nanos
            roundtrip_timestamp(19898323200, 2359807);
        }
    }
}

fn eq<T>(method: &str, expected: T, actual: T)
where
    T: PartialEq<T> + std::fmt::Debug,
{
    if expected != actual {
        panic!("{}: expected {:?}, got {:?}", method, expected, actual)
    }
}
