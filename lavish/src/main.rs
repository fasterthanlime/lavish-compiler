#![warn(clippy::all)]

use clap::{App, Arg, SubCommand};
use std::path::Path;
use std::rc::Rc;

mod ast;
mod checker;
mod codegen;
mod parser;
mod printer;

const VERSION: &str = "0.2.0";

fn main() {
    let matches = App::new("Lavish")
        .version(VERSION)
        .author("Amos Wenger <amoswenger@gmail.com>")
        .about("A service definition file compiler")
        .subcommand(
            SubCommand::with_name("build").arg(
                Arg::with_name("workspace")
                    .help("The workspace to compile")
                    .required(true)
                    .index(1),
            ),
        )
        .get_matches();

    match matches.subcommand() {
        ("build", Some(cmd)) => {
            let workspace_path = Path::new(cmd.value_of("workspace").unwrap());
            let workspace = parse_workspace(workspace_path).unwrap();
            codegen::codegen(&workspace).unwrap();
        }
        _ => {
            println!("{}", matches.usage());
            std::process::exit(1);
        }
    };
}

use simple_error::SimpleError;

fn parse_workspace(workspace_path: &Path) -> Result<ast::Workspace, Box<dyn std::error::Error>> {
    let rules_path = workspace_path.join("lavish-rules");
    if !rules_path.exists() {
        return Err(Box::new(SimpleError::new(format!(
            "{:?}: not a workspace (does not contain a 'lavish-rules' file)",
            workspace_path
        ))));
    }

    let source = parser::Source::new(&rules_path)?;
    let source = Rc::new(source);
    let rules = parser::parse_rules(source);
    unimplemented!()

    // let mut modules: Vec<ast::Module> = Vec::new();
    // let module = parser::parse(source.clone())?;

    // checker::check(&module)?;
    // modules.push(module);
    // Ok(modules)
}
