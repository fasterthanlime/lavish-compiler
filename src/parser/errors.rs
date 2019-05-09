use colored::*;
use nom::{
    error::{VerboseError, VerboseErrorKind},
    Err,
};
use std::fmt;
use std::iter::repeat;
use std::rc::Rc;

use super::super::ast;
use super::super::parser;
use parser::Span;

#[derive(Debug)]
pub enum Error {
    IO(std::io::Error),
    Source(SourceError),
    Unknown(UnknownError),
}

impl<'a> fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::IO(e) => write!(f, "{}", e),
            Error::Source(e) => write!(f, "{:#?}", e),
            Error::Unknown(_) => write!(f, "unknown error"),
        }
    }
}

impl std::error::Error for Error {}

pub struct SourceError {
    inner: VerboseError<parser::Span>,
}

impl<'a> fmt::Debug for SourceError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        print_errors(f, &self.inner)
    }
}

pub struct UnknownError {
    source: Rc<Source>,
}

impl fmt::Debug for UnknownError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "An unknown parsing error occured in {}",
            self.source.name()
        )
    }
}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Error::IO(e)
    }
}

pub struct Source {
    pub input: String,
    name: String,
    pub lines: Vec<String>,
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
}

pub fn parse<'a>(source: Rc<Source>) -> Result<ast::Module, Error> {
    let span = parser::Span {
        source: source.clone(),
        offset: 0,
        len: source.input.len(),
    };
    let res = parser::module::<VerboseError<parser::Span>>(span);
    match res {
        Err(Err::Error(e)) | Err(Err::Failure(e)) => Err(Error::Source(SourceError { inner: e })),
        Err(_) => Err(Error::Unknown(UnknownError {
            source: source.clone(),
        })),
        Ok((_, module)) => Ok(module),
    }
}

impl<'a> Source {
    pub fn name(&'a self) -> &'a str {
        &self.name
    }
}

pub struct Diagnostic<'a> {
    pos: Position,
    caret_color: Color,
    prefix: &'a str,
    message: String,
}

pub struct DiagnosticBuilder<'a> {
    pos: Position,
    caret_color: Color,
    prefix: &'a str,
    message: Option<String>,
}

const EMPTY_PREFIX: &'static str = "";

impl<'a> DiagnosticBuilder<'a> {
    pub fn new(pos: Position) -> Self {
        Self {
            pos: pos.clone(),
            caret_color: Color::Blue,
            prefix: EMPTY_PREFIX,
            message: None,
        }
    }

    pub fn caret_color(mut self, caret_color: Color) -> Self {
        self.caret_color = caret_color;
        self
    }

    pub fn prefix(mut self, prefix: &'a str) -> Self {
        self.prefix = prefix;
        self
    }

    pub fn message(mut self, message: String) -> Self {
        self.message = Some(message);
        self
    }

    pub fn build(self) -> Diagnostic<'a> {
        Diagnostic {
            pos: self.pos,
            caret_color: self.caret_color,
            prefix: self.prefix,
            message: self.message.unwrap_or_else(|| "".into()),
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
    pub fn print(self) {
        self.build().print()
    }

    pub fn write(self, f: &mut fmt::Formatter) -> fmt::Result {
        self.build().write(f)
    }
}

impl<'a> fmt::Display for Diagnostic<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let pos = &self.pos;
        let caret_color = self.caret_color;
        let prefix = self.prefix;
        let message = &self.message;

        let loc = format!(
            "{}:{}:{}:",
            pos.span.source.name(),
            pos.line + 1,
            pos.column + 1
        );
        writeln!(f, "{}{} {}", prefix, loc.bold(), message)?;
        let text_line = &pos.span.source.lines[pos.line];
        writeln!(f, "{}{}", prefix, text_line.dimmed())?;

        writeln!(
            f,
            "{}{}{}{}",
            prefix,
            repeat(' ').take(pos.column).collect::<String>(),
            "^".color(caret_color).bold(),
            repeat('~')
                .take(std::cmp::min(
                    match pos.span.len {
                        0 => 0,
                        x => x - 1,
                    },
                    text_line.len() - pos.column
                ))
                .collect::<String>()
                .color(caret_color)
                .bold()
        )?;
        Ok(())
    }
}

#[derive(Clone)]
pub struct Position {
    pub span: Span,
    pub line: usize,
    pub column: usize,
}

impl Position {
    fn diag<'a>(&self, message: String) -> DiagnosticBuilder<'a> {
        DiagnosticBuilder::new(self.clone()).message(message)
    }

    pub fn diag_info<'a>(&self, message: String) -> DiagnosticBuilder<'a> {
        self.diag(message).caret_color(Color::Blue)
    }

    pub fn diag_err<'a>(&self, message: String) -> DiagnosticBuilder<'a> {
        self.diag(message).caret_color(Color::Red)
    }
}

pub fn print_errors(f: &mut fmt::Formatter, e: &VerboseError<Span>) -> fmt::Result {
    let mut errors = e.errors.clone();
    errors.reverse();

    writeln!(f)?;
    for (span, kind) in errors.iter() {
        let pos = span.position();

        match kind {
            VerboseErrorKind::Char(c) => {
                pos.diag_err(format!(
                    "expected '{}', found {}",
                    c,
                    span.chars().next().unwrap()
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
