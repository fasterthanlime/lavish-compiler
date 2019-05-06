use colored::*;
use nom::{
    error::{VerboseError, VerboseErrorKind},
    Offset,
};
use std::iter::repeat;

use super::super::ast;

pub struct Source<'a> {
    name: String,
    pub input: &'a str,
    lines: Vec<String>,
}

impl<'a> Source<'a> {
    pub fn new(input_name: &'a str, input: &'a str) -> Self {
        Self {
            name: input_name.replace("./", ""),
            input,
            lines: input.lines().map(String::from).collect::<Vec<_>>(),
        }
    }
}

pub struct Position<'a> {
    source: &'a Source<'a>,
    line: usize,
    column: usize,
}

impl<'a> Source<'a> {
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

impl<'a> Position<'a> {
    pub fn print_message(&self, message: &str) {
        self.print_colored_message(Color::Blue, message);
    }

    pub fn print_colored_message(&self, caret_color: Color, message: &str) {
        self.print_colored_message_with_prefix(caret_color, "", message)
    }

    pub fn print_colored_message_with_prefix(
        &self,
        caret_color: Color,
        prefix: &str,
        message: &str,
    ) {
        let loc = format!(
            "{}:{}:{}:",
            self.source.name(),
            self.line + 1,
            self.column + 1
        );
        println!("{}{} {}", prefix, loc.bold(), message);
        println!("{}{}", prefix, &self.source.lines[self.line].dimmed());

        print!(
            "{}{}",
            prefix,
            repeat(' ').take(self.column).collect::<String>()
        );
        println!("{}", "^".color(caret_color).bold());
    }
}

pub fn print_errors<'a>(source: &Source<'a>, e: VerboseError<&str>) {
    let mut errors = e.errors.clone();
    errors.reverse();

    println!();
    for (slice, kind) in errors.iter() {
        let loc = ast::Loc { slice };
        let pos = source.position(&loc);

        match kind {
            VerboseErrorKind::Char(c) => {
                let error_msg =
                    format!("expected '{}', found {}", c, slice.chars().next().unwrap());
                pos.print_colored_message(Color::Red, &error_msg);
            }
            VerboseErrorKind::Context(s) => {
                let context_msg = format!("In {}", s);
                pos.print_colored_message(Color::Blue, &context_msg);
            }
            VerboseErrorKind::Nom(ek) => {
                let msg = format!("parsing error: {}", &format!("{:#?}", ek).red().bold());
                pos.print_colored_message(Color::Red, &msg);
            }
        }
    }
}
