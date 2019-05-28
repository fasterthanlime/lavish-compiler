#![warn(clippy::all)]

use clap::{App, Arg, SubCommand};
use std::collections::HashMap;
use std::path::Path;

mod ast;
mod checker;
mod codegen;
mod parser;
mod printer;

const VERSION: &str = "0.2.0";

fn main() {
    env_logger::init();

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

    let rules = {
        let source = parser::Source::from_path(&rules_path)?;
        parser::parse_rules(source)?
    };
    println!("rules = {:#?}", rules);

    let mut workspace = ast::Workspace {
        dir: workspace_path.into(),
        rules,
        members: HashMap::new(),
    };

    println!("{} builds", workspace.rules.builds.len());
    for build in &workspace.rules.builds {
        let name = build.name.text.to_string();
        let source_path = workspace.resolve(&name)?;
        println!("Parsing {} from {:?}", name, source_path);
        let source = parser::Source::from_path(&source_path)?;
        let schema = parser::parse_schema(source)?;

        checker::check(&schema)?;

        workspace.members.insert(
            name.clone(),
            ast::WorkspaceMember {
                name,
                build: Some(build.clone()),
                imports: Vec::new(),
                schema: Some(schema),
            },
        );
    }

    Ok(workspace)
}
