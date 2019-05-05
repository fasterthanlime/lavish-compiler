use nom::{error::VerboseError, Err};
use std::fs::File;
use std::io::Read;

mod ast;
mod parser;

fn main() {
    let mut data = String::new();
    {
        let mut f = File::open("./samples/test.lavish").unwrap();
        f.read_to_string(&mut data).unwrap();
    }

    println!("================ input =================");
    println!("{}", data);
    println!("========================================");
    match parser::root::<VerboseError<&str>>(&data) {
        Err(Err::Error(e)) | Err(Err::Failure(e)) => {
            println!(
                "verbose errors - `root::<VerboseError>(data)`:\n{}",
                parser::convert_error(&data, e)
            );
        }
        Ok(res) => println!("Parsed: {:#?}", res),
        _ => println!("Something else happened :o"),
    }
}
