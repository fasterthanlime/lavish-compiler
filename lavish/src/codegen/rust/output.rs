use super::super::ast;
use crate::codegen::Result;
use std::fmt;
use std::io;

const INDENT_WIDTH: usize = 4;

pub struct Scope<'a> {
    writer: &'a mut io::Write,
    indent: usize,
    state: ScopeState,
}

enum ScopeState {
    NeedIndent,
    Indented,
}

impl<'a> Scope<'a> {
    pub fn new(writer: &'a mut io::Write) -> Self {
        Self {
            writer,
            indent: 0,
            state: ScopeState::NeedIndent,
        }
    }

    pub fn line<S>(&mut self, line: S)
    where
        S: AsRef<str>,
    {
        writeln!(self.writer, "{}{}", " ".repeat(self.indent), line.as_ref()).unwrap();
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
        f(&mut s)?;
        Ok(())
    }

    pub fn scope(&mut self) -> Scope {
        Scope {
            writer: self.writer,
            indent: self.indent + INDENT_WIDTH,
            state: ScopeState::NeedIndent,
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
            let mut s = Scope::new(&mut buf);
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
