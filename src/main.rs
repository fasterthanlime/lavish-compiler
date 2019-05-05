use colored::*;
use nom::{error::VerboseError, Err, Offset};
use std::fs::File;
use std::io::Read;

mod ast;
mod parser;

struct Visitor<'a> {
    source: &'a parser::Source<'a>,
}

trait Visitable {
    fn visit<'a>(&self, v: &'a Visitor<'a>);
}

impl<'a> Visitable for ast::NamespaceDecl<'a> {
    fn visit(&self, v: &Visitor) {
        println!("Visiting namespace {}", self.name);
        for (_, ns) in &self.namespaces {
            ns.visit(v);
        }
        for (_, s) in &self.structs {
            s.visit(v);
        }
        for (_, f) in &self.functions {
            f.visit(v);
        }
    }
}

impl<'a> Visitable for ast::StructDecl<'a> {
    fn visit(&self, v: &Visitor) {
        println!("Visiting struct {}", self.name);
        for p in &self.fields {
            p.visit(v);
        }
    }
}

impl<'a> Visitable for ast::FunctionDecl<'a> {
    fn visit(&self, v: &Visitor) {
        println!("Visiting function {}", self.name);
        for p in &self.params {
            p.visit(v);
        }
        for p in &self.results {
            p.visit(v);
        }
    }
}

impl<'a> Visitable for ast::Field<'a> {
    fn visit(&self, v: &Visitor) {
        let pos = v.source.position(&self.loc);
        let mut comment = ColoredString::default();
        if let Some(c) = self.comment.as_ref() {
            comment = format!(
                " — {}",
                c.lines.iter().map(|x| x.clone()).collect::<String>()
            )
            .blue();
        };

        pos.print_message(&format!(
            "field {}, of type {}{}",
            self.name.yellow(),
            self.typ.green(),
            comment,
        ));
    }
}

fn main() {
    let input_name = "./samples/test.lavish";
    let mut data = String::new();
    {
        let mut f = File::open(input_name).unwrap();
        f.read_to_string(&mut data).unwrap();
    }

    match parser::root::<VerboseError<&str>>(&data) {
        Err(Err::Error(e)) | Err(Err::Failure(e)) => {
            println!();
            parser::print_errors(input_name, &data, e);
        }
        Ok((_, output)) => {
            // println!("Parsed: {:#?}", output);
            let source = parser::Source::new(input_name, &data);
            let v = Visitor { source: &source };
            for ns in output.iter() {
                ns.visit(&v)
            }
        }
        _ => println!("Something else happened :o"),
    }
}
