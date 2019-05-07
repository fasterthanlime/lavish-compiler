use colored::*;
use nom::{
    error::{VerboseError, VerboseErrorKind},
    Err, Offset,
};
use std::fmt;
use std::iter::repeat;

use super::super::ast;
use super::super::parser;

#[derive(Debug)]
pub enum Error<'a> {
    IO(std::io::Error),
    Source(SourceError<'a>),
    Unknown(UnknownError<'a>),
}

pub struct SourceError<'a> {
    source: &'a Source,
    inner: VerboseError<&'a str>,
}

impl<'a> fmt::Debug for SourceError<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        print_errors(f, self.source, &self.inner)
    }
}

pub struct UnknownError<'a> {
    source: &'a Source,
}

impl<'a> fmt::Debug for UnknownError<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "An unknown parsing error occured in {}",
            self.source.name()
        )
    }
}

impl<'a> From<std::io::Error> for Error<'a> {
    fn from(e: std::io::Error) -> Self {
        Error::IO(e)
    }
}

pub struct Source {
    pub input: String,
    name: String,
    lines: Vec<String>,
}

impl Source {
    pub fn new(input_name: &str) -> Result<Self, std::io::Error> {
        use std::fs::File;
        use std::io::Read;

        let mut input = String::new();
        {
            let mut f = File::open(input_name)?;
            f.read_to_string(&mut input)?;
        }
        let lines = input.lines().map(String::from).collect::<Vec<_>>();

        Ok(Self {
            name: input_name.replace("./", ""),
            input,
            lines,
        })
    }

    pub fn parse2<'a>(source: std::rc::Rc<Self>) -> Result<ast::Module2, Error<'a>> {
        Ok(ast::Module2 { source })
    }

    pub fn parse<'a>(&'a self) -> Result<ast::Module<'a>, Error<'a>> {
        let res = parser::module::<VerboseError<&'a str>>(&self.input);
        match res {
            Err(Err::Error(e)) | Err(Err::Failure(e)) => Err(Error::Source(SourceError {
                inner: e,
                source: self,
            })),
            Err(_) => Err(Error::Unknown(UnknownError { source: self })),
            Ok((_, module)) => Ok(module),
        }
    }
}

pub struct Position<'a> {
    source: &'a Source,
    line: usize,
    column: usize,
}

impl<'a> Source {
    pub fn name(&'a self) -> &'a str {
        &self.name
    }

    pub fn position(&self, loc: &'a ast::Loc<'a>) -> Position {
        let mut offset = self.input.offset(loc.slice);
        let mut line = 0;
        let mut column = 0;

        for (j, l) in self.lines.iter().enumerate() {
            if offset <= l.len() {
                line = j;
                column = offset;
                break;
            } else {
                // 1 accounts for the '\n'
                offset = offset - l.len() - 1;
            }
        }

        Position {
            source: &self,
            line,
            column,
        }
    }
}

use derive_builder::*;

#[derive(Builder)]
#[builder(default)]
pub struct Diagnostic<'a> {
    pos: Option<&'a Position<'a>>,
    caret_color: Color,
    prefix: &'a str,
    message: String,
}

const EMPTY_PREFIX: &'static str = "";

impl<'a> Default for Diagnostic<'a> {
    fn default() -> Self {
        Self {
            pos: None,
            caret_color: Color::Blue,
            prefix: EMPTY_PREFIX,
            message: "".into(),
        }
    }
}

impl<'a> Diagnostic<'a> {
    pub fn print(&self) {
        print!("{}", self)
    }

    pub fn write(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self)
    }
}

impl<'a> DiagnosticBuilder<'a> {
    pub fn print(&self) {
        self.build().unwrap().print()
    }

    pub fn write(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.build().unwrap().write(f)
    }
}

impl<'a> fmt::Display for Diagnostic<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.pos {
            Some(pos) => {
                let caret_color = self.caret_color;
                let prefix = self.prefix;
                let message = &self.message;

                let loc = format!("{}:{}:{}:", pos.source.name(), pos.line + 1, pos.column + 1);
                writeln!(f, "{}{} {}", prefix, loc.bold(), message)?;
                writeln!(f, "{}{}", prefix, &pos.source.lines[pos.line].dimmed())?;

                writeln!(
                    f,
                    "{}{}{}",
                    prefix,
                    repeat(' ').take(pos.column).collect::<String>(),
                    "^".color(caret_color).bold()
                )?;
            }
            None => {
                writeln!(f, "(no position information): {}", self.message)?;
            }
        }
        Ok(())
    }
}

impl<'a> Position<'a> {
    fn diag(&'a self, message: String) -> DiagnosticBuilder<'a> {
        let mut builder = DiagnosticBuilder::default();
        builder.pos(Some(self));
        builder.message(message);
        builder
    }

    pub fn diag_info(&'a self, message: String) -> DiagnosticBuilder<'a> {
        let mut builder = self.diag(message);
        builder.caret_color(Color::Blue);
        builder
    }

    pub fn diag_err(&'a self, message: String) -> DiagnosticBuilder<'a> {
        let mut builder = self.diag(message);
        builder.caret_color(Color::Red);
        builder
    }
}

pub fn print_errors(
    f: &mut fmt::Formatter,
    source: &Source,
    e: &VerboseError<&str>,
) -> fmt::Result {
    let mut errors = e.errors.clone();
    errors.reverse();

    writeln!(f)?;
    for (slice, kind) in errors.iter() {
        let loc = ast::Loc { slice };
        let pos = source.position(&loc);

        match kind {
            VerboseErrorKind::Char(c) => {
                pos.diag_err(format!(
                    "expected '{}', found {}",
                    c,
                    slice.chars().next().unwrap()
                ))
                .write(f)?;
            }
            VerboseErrorKind::Context(s) => {
                pos.diag_info(format!("In {}", s)).write(f)?;
            }
            VerboseErrorKind::Nom(ek) => {
                pos.diag_err(format!(
                    "parsing error: {}",
                    &format!("{:#?}", ek).red().bold()
                ))
                .write(f)?;
            }
        }
    }

    Ok(())
}
