use colored::*;
use std::process::Command;
use std::{env, fs, path};

fn status<S: Into<String>>(s: S) {
    println!("{}", s.into().blue());
}

fn task<S: Into<String>>(s: S) {
    println!("{}", s.into().yellow());
}

struct Context {
    tests_dir: path::PathBuf,
    compiler_path: path::PathBuf,
}

fn main() {
    std::env::set_var("RUST_BACKTRACE", "1");

    let cwd = env::current_dir().unwrap();
    let tests_dir = cwd.join("tests");

    if !tests_dir.exists() {
        panic!("test dir {:?} does not exist (wrong working directory?)")
    }

    status("Building lavish compiler...");
    run(Command::new("cargo").args(&["build"]));

    let context = Context {
        tests_dir: tests_dir.into(),
        compiler_path: cwd.join("target").join("debug").join("lavish").into(),
    };

    context.run_codegen_tests();
    context.run_integration_tests();

    status("All done!")
}

const LAVISH_REVISION: &str = "ced097658a9246bfc2d7d68f03e97ce0ca98c4d4";
const RUST_CODEGEN_CARGO_TEMPLATE: &str = r#"
[package]
name = "test"
version = "0.1.0"
edition = "2018"

[dependencies.lavish]
git = "https://github.com/lavish-lang/lavish-rs"
rev = "{{LAVISH_REVISION}}"
"#;

const RUST_CODEGEN_LAVISH_RULES_TEMPLATE: &str = r#"
target rust {
    wrapper = lib
}

{{BUILDS}}
"#;

struct CodegenCase {
    name: String,
    schema_path: path::PathBuf,
}

impl Context {
    fn run_codegen_tests(&self) {
        let codegen_dir = self.tests_dir.join("codegen");
        task("Codegen tests");

        let mut cases = Vec::<CodegenCase>::new();
        for schema_path in codegen_dir
            .read_dir()
            .expect("rust codegen tests dir should exist")
        {
            let schema_path = schema_path.unwrap().path();
            let extension = match schema_path.extension() {
                Some(x) => x,
                None => continue,
            };
            if extension != "lavish" {
                continue;
            }

            let name = schema_path.file_stem().unwrap().to_string_lossy();
            cases.push(CodegenCase {
                name: name.into(),
                schema_path,
            });
        }
        status(format!("Found {} codegen tests", cases.len()));

        // Rust
        {
            task("Rust codegen...");

            let tmp_dir = codegen_dir.join(".tmp");
            let target_dir = tmp_dir.join("rust_target");
            let harness_dir = tmp_dir.join("rust_harness");

            if harness_dir.exists() {
                fs::remove_dir_all(&harness_dir).unwrap();
            }
            fs::create_dir_all(&harness_dir).unwrap();

            let cargo_path = harness_dir.join("Cargo.toml");
            let cargo_template =
                RUST_CODEGEN_CARGO_TEMPLATE.replace("{{LAVISH_REVISION}}", LAVISH_REVISION);
            fs::write(&cargo_path, cargo_template).unwrap();

            let src_dir = harness_dir.join("src");
            fs::create_dir_all(&src_dir).unwrap();

            {
                let mut builds = Vec::<String>::new();
                for case in &cases {
                    builds.push(format!(
                        "build {name} from {path:?}",
                        name = case.name.replace("-", "_"),
                        path = case.schema_path
                    ));
                }

                let rules_path = src_dir.join("lavish-rules");
                let rules_template =
                    RUST_CODEGEN_LAVISH_RULES_TEMPLATE.replace("{{BUILDS}}", &builds.join("\n"));
                fs::write(&rules_path, &rules_template).unwrap();
            }

            run(Command::new(&self.compiler_path).args(&["build", &src_dir.to_string_lossy()]));

            run(Command::new("cargo")
                .args(&["check", "--manifest-path", &cargo_path.to_string_lossy()])
                .env("CARGO_TARGET_DIR", target_dir)
                .current_dir(&harness_dir));
        }
    }

    fn run_integration_tests(&self) {
        println!("Should run integration tests!");
    }
}

fn run(c: &mut Command) {
    println!("{}", format!("{:?}", c).blue());
    c.status().unwrap();
}
