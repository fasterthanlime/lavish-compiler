use super::super::ast;
use super::Error;
use indexmap::IndexMap;

struct Context<'a> {
    namespaces: IndexMap<&'a str, Namespace<'a>>,
}

impl<'a> Context<'a> {
    fn new() -> Self {
        Self {
            namespaces: IndexMap::new(),
        }
    }

    fn visit_toplevel_ns(&mut self, decl: &'a ast::NamespaceDecl) {
        let k = decl.name.text.as_ref();

        let mut ns = Namespace::new(decl);
        self.visit_ns("", &mut ns);

        if let Some(original_ns) = self.namespaces.get_mut(k) {
            original_ns.merge(ns)
        } else {
            self.namespaces.insert(k, ns);
        }
    }

    fn visit_ns(&mut self, prefix: &str, ns: &mut Namespace<'a>) {
        let decl = &ns.decl;
        let prefix = format!("{}{}.", prefix, ns.name());

        for decl in &decl.functions {
            let ff = Fun::<'a> {
                decl,
                full_name: format!("{}{}", prefix, decl.name.text),
            };
            ns.funs.insert(&decl.name.text, ff);
        }

        for decl in &decl.namespaces {
            let mut child = Namespace::new(decl);
            self.visit_ns(&prefix, &mut child);
            ns.children.insert(decl.name.text.as_ref(), child);
        }
    }

    fn funs(&self) -> impl Iterator<Item = &Fun<'a>> {
        self.namespaces.values().map(|ns| ns.funs()).flatten()
    }
}

struct Namespace<'a> {
    decl: &'a ast::NamespaceDecl,
    children: IndexMap<&'a str, Namespace<'a>>,

    funs: IndexMap<&'a str, Fun<'a>>,
}

impl<'a> Namespace<'a> {
    fn new(decl: &'a ast::NamespaceDecl) -> Self {
        Namespace {
            decl,
            children: IndexMap::new(),
            funs: IndexMap::new(),
        }
    }

    fn funs(&self) -> impl Iterator<Item = &Fun<'a>> {
        self.children
            .values()
            .map(|ns| ns.funs())
            .flatten()
            .chain(self.funs.values())
    }

    fn merge(&mut self, rhs: Self) {
        for (k, v) in rhs.children {
            if let Some(sv) = self.children.get_mut(k) {
                sv.merge(v)
            } else {
                self.children.insert(k, v);
            }
        }

        for (k, v) in rhs.funs {
            self.funs.insert(k, v);
        }
    }

    fn name(&self) -> &'a str {
        &self.decl.name.text
    }
}

struct Fun<'a> {
    #[allow(unused)]
    decl: &'a ast::FunctionDecl,
    full_name: String,
}

pub fn codegen<'a>(modules: &'a Vec<ast::Module>) -> Result<(), Error> {
    use std::fs::File;
    use std::io::Write;
    use std::path::Path;

    let p = Path::new("output/out.rs");
    std::fs::create_dir_all(p.parent().unwrap())?;
    let mut out = File::create(p).unwrap();

    write!(
        out,
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

    let mut ctx = Context::new();
    for module in modules {
        for decl in &module.namespaces {
            ctx.visit_toplevel_ns(decl);
        }
    }

    for fun in ctx.funs() {
        println!("Found {}", fun.full_name);

        write!(
            out,
            r#"
pub mod {} {{
    #[derive(Debug, Deserialize, Serialize)]
    pub struct Params
}}
        "#,
            fun.full_name
        )?;
    }

    println!("All done!");

    Ok(())
}
