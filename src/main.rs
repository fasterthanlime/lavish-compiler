use std::fs::File;
use std::io::Read;

mod ast;
mod parser;
mod printer;

fn main() {
    let input_name = "./samples/test.lavish";
    let mut data = String::new();
    {
        let mut f = File::open(input_name).unwrap();
        f.read_to_string(&mut data).unwrap();
    }
    let source = parser::Source::new(input_name, &data);

    match parser::parse(&source) {
        Err(e) => {
            parser::print_errors(&source, e);
        }
        Ok(module) => {
            let source = parser::Source::new(input_name, &data);
            let v = printer::Visitor::new(&source);
            v.visit(module);
        }
    }
}
