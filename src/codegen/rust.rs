use super::super::ast;
use super::Error;

struct FnName {
    full_name: String,
}

impl FnName {}

fn visit_ns<'a>(
    functions: &mut Vec<(FnName, &'a ast::FunctionDecl)>,
    prefix: &str,
    ns: &'a ast::NamespaceDecl,
) {
    let prefix = format!("{}{}.", prefix, ns.name.text);

    for f in &ns.functions {
        let name = FnName {
            full_name: format!("{}{}", prefix, f.name.text),
        };
        functions.push((name, f))
    }

    for ns in &ns.namespaces {
        visit_ns(functions, &prefix, ns);
    }
}

pub fn codegen<'a>(modules: &'a Vec<ast::Module>) -> Result<(), Error> {
    let mut functions: Vec<(FnName, &ast::FunctionDecl)> = Vec::new();
    use std::fs::File;
    use std::io::Write;
    use std::path::Path;

    let p = Path::new("output/out.rs");
    std::fs::create_dir_all(p.parent().unwrap())?;
    let mut f = File::create(p).unwrap();

    write!(
        f,
        "{}",
        r#"
#[derive(Debug)]
enum Message {
    Request {
        parent: Option<u32>,
        id: u32,
        params: Params,
    },
    #[allow(unused)]
    Response {
        id: u32,
        error: Option<String>,
        results: Results,
    },
}
"#
    )?;

    for module in modules {
        for ns in &module.namespaces {
            visit_ns(&mut functions, "", ns);
        }
    }

    for (name, _f) in functions {
        println!("Found {}", name.full_name);
    }

    Ok(())
}
