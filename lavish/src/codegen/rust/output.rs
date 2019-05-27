use super::super::ast;
use std::cell::RefCell;
use std::fs::File;
use std::io::{self, Write};

const INDENT_WIDTH: usize = 4;

pub struct Output {
    writer: RefCell<io::BufWriter<File>>,
}

impl Output {
    pub fn new(file: File) -> Self {
        Self {
            writer: RefCell::new(io::BufWriter::new(file)),
        }
    }
}

impl<'a> io::Write for &'a Output {
    fn write(&mut self, b: &[u8]) -> io::Result<usize> {
        let mut w = self.writer.borrow_mut();
        w.write(b)
    }

    fn flush(&mut self) -> io::Result<()> {
        let mut w = self.writer.borrow_mut();
        w.flush()
    }
}

pub struct Scope<'a> {
    output: &'a Output,
    indent: usize,
}

impl<'a> Scope<'a> {
    pub fn new(output: &'a Output) -> Self {
        Self { output, indent: 0 }
    }

    pub fn line(&self, line: &str) {
        writeln!(
            self.output.writer.borrow_mut(),
            "{}{}",
            " ".repeat(self.indent),
            line
        )
        .unwrap();
    }

    pub fn comment(&self, comment: &Option<ast::Comment>) {
        if let Some(comment) = comment.as_ref() {
            for line in &comment.lines {
                self.line(&format!("// {}", line))
            }
        }
    }

    pub fn def_struct(&self, name: &str, f: &Fn(&Scope)) {
        self.line("#[derive(Serialize, Deserialize, Debug)]");
        self.line(&format!("pub struct {} {{", name));
        self.in_scope(f);
        self.line("}");
    }

    pub fn in_scope(&self, f: &Fn(&Scope)) {
        let s = self.scope();
        f(&s);
    }

    pub fn scope(&self) -> Scope {
        Scope {
            output: &self.output,
            indent: self.indent + INDENT_WIDTH,
        }
    }
}
