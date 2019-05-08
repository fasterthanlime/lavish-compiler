use clap::{App, Arg, SubCommand};

mod ast;
mod checker;
mod codegen;
mod parser;
mod printer;

const VERSION: &str = "0.1.0";

fn main() {
    let matches = App::new("Lavish")
        .version(VERSION)
        .author("Amos Wenger <amoswenger@gmail.com>")
        .about("A service definition file compiler")
        .subcommand(
            SubCommand::with_name("check").arg(
                Arg::with_name("input")
                    .help("The file to check")
                    .required(true)
                    .index(1),
            ),
        )
        .subcommand(
            SubCommand::with_name("codegen").arg(
                Arg::with_name("input")
                    .help("The file to compile")
                    .required(true)
                    .index(1),
            ),
        )
        .get_matches();

    match matches.subcommand() {
        ("check", Some(cmd)) => {
            let modules = check(cmd.value_of("input").unwrap()).unwrap();
            for module in modules {
                printer::print(&module);
            }
        }
        ("compile", Some(cmd)) => {
            let modules = check(cmd.value_of("input").unwrap()).unwrap();
            codegen::codegen(&modules, codegen::Target::Rust).unwrap();
        }
        _ => {
            println!("{}", matches.usage());
            std::process::exit(1);
        }
    };
}

fn check(input_name: &str) -> Result<Vec<ast::Module>, Box<dyn std::error::Error>> {
    let mut modules: Vec<ast::Module> = Vec::new();

    let source = parser::Source::new(input_name)?;
    let source = std::rc::Rc::new(source);
    let module = parser::parse(source.clone())?;

    checker::check(&module)?;
    modules.push(module);
    Ok(modules)
}
