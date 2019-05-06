use colored::*;
use std::fs::File;
use std::io::Read;

use clap::{App, Arg, SubCommand};

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
            }
        }
        _ => {
            println!("{}", matches.usage());
            std::process::exit(1);
        }
    };
}
