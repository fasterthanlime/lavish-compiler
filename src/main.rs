use nom::{error::VerboseError, Err};

mod ast;
mod parser;

fn main() {
    let data = "
namespace itchio {
    // Woops
    struct Profile {}
}
";
    println!("================ input =================");
    println!("{}", data);
    println!("========================================");
    match parser::root::<VerboseError<&str>>(data) {
        Err(Err::Error(e)) | Err(Err::Failure(e)) => {
            println!(
                "verbose errors - `root::<VerboseError>(data)`:\n{}",
                parser::convert_error(data, e)
            );
        }
        Ok(res) => println!("Parsed: {:#?}", res),
        _ => println!("Something else happened :o"),
    }
}
