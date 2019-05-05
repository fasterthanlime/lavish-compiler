use colored::*;
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
        v.source.position(&self.loc).print_message(&format!(
            "namespace {}{}",
            self.name.yellow(),
            format_comment(&self.comment),
        ));
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
        v.source.position(&self.loc).print_message(&format!(
            "struct {}{}",
            self.name.yellow(),
            format_comment(&self.comment),
        ));
        for p in &self.fields {
            p.visit(v);
        }
    }
}

impl<'a> Visitable for ast::FunctionDecl<'a> {
    fn visit(&self, v: &Visitor) {
        v.source.position(&self.loc).print_message(&format!(
            "function {}{}",
            self.name.yellow(),
            format_comment(&self.comment),
        ));
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
        v.source.position(&self.loc).print_message(&format!(
            "field {}, of type {}{}",
            self.name.yellow(),
            self.typ.green(),
            format_comment(&self.comment),
        ));
    }
}

fn format_comment(comment: &Option<ast::Comment>) -> ColoredString {
    let mut result = ColoredString::default();
    if let Some(comment) = comment.as_ref() {
        result = format!(
            " — {}",
            comment.lines.iter().map(|x| x.clone()).collect::<String>()
        )
        .blue();
    };
    result
}

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
        Ok(output) => {
            // println!("Parsed: {:#?}", output);
            let source = parser::Source::new(input_name, &data);
            let v = Visitor { source: &source };
            for ns in output.iter() {
                ns.visit(&v)
            }
        }
    }
}
