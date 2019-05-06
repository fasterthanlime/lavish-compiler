use clap::{App, Arg, SubCommand};
use colored::*;

mod ast;
mod checker;
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
                    .help("The file to compile")
                    .required(true)
                    .index(1),
            ),
        )
        .get_matches();

    match matches.subcommand() {
        ("check", Some(cmd)) => {
            let input_name = cmd.value_of("input").unwrap();
            let source = parser::Source::new(input_name).unwrap();
            let module = source.parse().unwrap();

            checker::check(&source, &module).unwrap_or_else(|e| {
                println!(
                    "{} found {} errors, existing",
                    "error:".red().bold(),
                    e.num_errors
                );
                std::process::exit(1);
            });
            printer::print(&source, &module);
        }
        _ => {
            println!("{}", matches.usage());
            std::process::exit(1);
        }
    };
}
