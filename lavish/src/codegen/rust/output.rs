// TODO: remove at some point
#![allow(unused)]

use super::super::ast;
use crate::codegen::Result;
use std::fmt::{self, Display, Write};
use std::io::{self, BufWriter};

const INDENT_WIDTH: usize = 4;

pub struct Writer<W> {
    writer: W,
}

impl<W> Writer<W>
where
    W: std::io::Write,
{
    pub fn new(writer: W) -> Self {
        Self { writer }
    }

    #[allow(unused)]
    pub fn into_inner(self) -> W {
        self.writer
    }
}

impl<W> fmt::Write for Writer<W>
where
    W: std::io::Write,
{
    fn write_str(&mut self, s: &str) -> fmt::Result {
        write!(self.writer, "{}", s).map_err(|_| fmt::Error {})
    }
}

pub struct Scope<'a> {
    writer: &'a mut fmt::Write,
    indent: usize,
    state: ScopeState,
}

enum ScopeState {
    NeedIndent,
    Indented,
}

impl<'a> Scope<'a> {
    pub fn new(writer: &'a mut fmt::Write) -> Self {
        Self {
            writer,
            indent: 0,
            state: ScopeState::NeedIndent,
        }
    }

    pub fn writer<W>(w: W) -> Writer<BufWriter<W>>
    where
        W: io::Write,
    {
        Writer::new(BufWriter::new(w))
    }

    pub fn lf(&mut self) {
        writeln!(self).unwrap();
    }

    pub fn line<D>(&mut self, d: D)
    where
        D: Display,
    {
        self.write(d).lf()
    }

    pub fn write<D>(&mut self, d: D) -> &mut Self
    where
        D: Display,
    {
        write!(self, "{}", d).unwrap();
        self
    }

    pub fn comment(&mut self, comment: &Option<ast::Comment>) {
        if let Some(comment) = comment.as_ref() {
            for line in &comment.lines {
                self.line(format!("/// {}", line))
            }
        }
    }

    pub fn def_struct<F>(&mut self, name: &str, f: F) -> Result
    where
        F: Fn(&mut Scope) -> Result,
    {
        self.line("#[derive(Serialize, Deserialize, Debug)]");
        self.line(format!("pub struct {} {{", name));
        self.in_scope(f)?;
        self.line("}");
        Ok(())
    }

    pub fn in_scope<F>(&mut self, f: F) -> Result
    where
        F: Fn(&mut Scope) -> Result,
    {
        let mut s = self.scope();
        f(&mut s)
    }

    pub fn in_block<F>(&mut self, f: F) -> &mut Self
    where
        F: Fn(&mut Scope),
    {
        self.line(" {");
        {
            let mut s = self.scope();
            f(&mut s);
        }
        self.line("}");
        self
    }

    pub fn fmt<F>(writer: &'a mut fmt::Write, f: F) -> std::fmt::Result
    where
        F: Fn(&mut Scope),
    {
        let mut s = Self::new(writer);
        f(&mut s);
        Ok(())
    }

    pub fn scope(&mut self) -> Scope {
        Scope {
            writer: self.writer,
            indent: self.indent + INDENT_WIDTH,
            state: ScopeState::NeedIndent,
        }
    }

    pub fn in_list<F>(&mut self, brackets: Brackets, f: F) -> &mut Self
    where
        F: Fn(&mut CommaList),
    {
        {
            let mut list = CommaList::new(self, brackets);
            f(&mut list);
        }
        self
    }

    pub fn in_parens_list<F>(&mut self, f: F) -> &mut Self
    where
        F: Fn(&mut CommaList),
    {
        self.in_list(Brackets::Round, f)
    }

    pub fn in_brackets<F>(&mut self, brackets: Brackets, f: F) -> &mut Self
    where
        F: Fn(&mut Scope),
    {
        {
            self.write(brackets.open());
            f(self);
            self.write(brackets.close());
        }
        self
    }

    pub fn in_parens<F>(&mut self, f: F) -> &mut Self
    where
        F: Fn(&mut Scope),
    {
        self.in_brackets(Brackets::Round, f)
    }
}

#[derive(Clone, Copy)]
pub enum Brackets {
    Round,
    Squares,
    Curly,
    Angle,
}

impl Brackets {
    pub fn pair(&self) -> (char, char) {
        match self {
            Brackets::Round => ('(', ')'),
            Brackets::Squares => ('[', ']'),
            Brackets::Curly => ('{', '}'),
            Brackets::Angle => ('<', '>'),
        }
    }

    pub fn open(&self) -> char {
        self.pair().0
    }

    pub fn close(&self) -> char {
        self.pair().1
    }
}

pub struct CommaList<'a: 'b, 'b> {
    scope: &'b mut Scope<'a>,
    brackets: Brackets,

    empty_list: bool,
    omit_empty: bool,
}

impl<'a: 'b, 'b> CommaList<'a, 'b> {
    pub fn new(scope: &'b mut Scope<'a>, brackets: Brackets) -> Self {
        Self {
            scope,
            brackets,
            empty_list: true,
            omit_empty: false,
        }
    }

    pub fn omit_empty(&mut self) {
        self.omit_empty = true;
    }

    pub fn item<D>(&mut self, item: D)
    where
        D: Display,
    {
        let s = &mut self.scope;
        if self.empty_list {
            s.write(self.brackets.open());
            self.empty_list = false
        } else {
            s.write(", ");
        }
        s.write(item);
    }
}

impl<'a, 'b> Drop for CommaList<'a, 'b> {
    fn drop(&mut self) {
        if self.empty_list {
            if self.omit_empty {
                return;
            }

            self.scope
                .write(self.brackets.open())
                .write(self.brackets.close());
        } else {
            self.scope.write(self.brackets.close());
        }
    }
}

impl<'a> fmt::Write for Scope<'a> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for (i, token) in s.split('\n').enumerate() {
            // each token is a string slice without newlines
            if i > 0 {
                // each token after the first one is preceded by a newline,
                // so let's write it out
                writeln!(self.writer).map_err(|_| fmt::Error {})?;
                self.state = ScopeState::NeedIndent;
            }

            if token.is_empty() {
                continue;
            }

            match self.state {
                ScopeState::NeedIndent => {
                    write!(self.writer, "{}", " ".repeat(self.indent))
                        .map_err(|_| fmt::Error {})?;
                    self.state = ScopeState::Indented
                }
                ScopeState::Indented => {}
            }
            write!(self.writer, "{}", token).map_err(|_| fmt::Error {})?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::Scope;
    use netbuf::Buf;
    use std::fmt::Write;

    #[test]
    fn test_scope() -> Result<(), Box<dyn std::error::Error + 'static>> {
        let mut buf = Buf::new();
        {
            let mut w = super::Writer::new(&mut buf);
            let mut s = Scope::new(&mut w);
            writeln!(s, "fn sample() {{")?;
            {
                let mut s = s.scope();
                writeln!(s, "let a = {{")?;
                {
                    let mut s = s.scope();
                    let val = 7;
                    writeln!(s, "let tmp = {val};", val = val)?;
                    writeln!(s, "// a blank line follows")?;
                    writeln!(s)?;
                    writeln!(s, "tmp + 3")?;
                }
                writeln!(s, "}};")?;
            }
            writeln!(s, "}}")?;
        }

        let s = std::str::from_utf8(buf.as_ref()).unwrap();
        assert_eq!(
            s,
            r#"fn sample() {
    let a = {
        let tmp = 7;
        // a blank line follows

        tmp + 3
    };
}
"#,
        );
        Ok(())
    }
}
