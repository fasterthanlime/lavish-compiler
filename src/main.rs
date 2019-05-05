use nom::{error::VerboseError, Err};
use std::fs::File;
use std::io::Read;

mod ast;
mod parser;

fn main() {
    let input_name = "./samples/test.lavish";
    let mut data = String::new();
    {
        let mut f = File::open(input_name).unwrap();
        f.read_to_string(&mut data).unwrap();
    }

    println!("================ input =================");
    println!("{}", data);
    println!("========================================");
    match parser::root::<VerboseError<&str>>(&data) {
        Err(Err::Error(e)) | Err(Err::Failure(e)) => {
            parser::print_errors(input_name, &data, e);
        }
        Ok(res) => println!("Parsed: {:#?}", res),
        _ => println!("Something else happened :o"),
    }
}
