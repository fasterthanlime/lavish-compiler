use colored::*;
use nom::{
    error::{VerboseError, VerboseErrorKind},
    Offset,
};
use std::iter::repeat;

use super::super::ast;

pub struct Source<'a> {
    name: String,
    input: &'a str,
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
        let loc = format!(
            "{}:{}:{}:",
            self.source.name(),
            self.line + 1,
            self.column + 1
        );
        println!("{} {}", loc.bold(), message);
        println!("{}", &self.source.lines[self.line].dimmed());
        print!("{}", repeat(' ').take(self.column).collect::<String>());
        println!("{}", "^".blue().bold());
    }
}

pub fn print_errors(input_name: &str, input: &str, e: VerboseError<&str>) {
    let input_name = input_name.replace("./", "");
    let lines: Vec<_> = input.lines().map(String::from).collect();

    let mut errors = e.errors.clone();
    errors.reverse();

    for (substring, kind) in errors.iter() {
        let mut offset = input.offset(substring);
        // result += &format!("offset: {:#?}\n", offset);

        let mut line = 0;
        let mut column = 0;

        for (j, l) in lines.iter().enumerate() {
            if offset <= l.len() {
                line = j;
                column = offset;
                break;
            } else {
                // 1 accounts for the '\n'
                offset = offset - l.len() - 1;
            }
        }

        let loc = format!("{}:{}:{}", input_name, line + 1, column + 1);
        let loc = loc.bold();

        let print_line = |highlight: bool| {
            let line = &lines[line];
            if highlight {
                print!("{}", &line[0..column].dimmed());
                print!("{}", &line[column..column + 1].red().bold());
                print!("{}\n", &line[column + 1..].dimmed());
            } else {
                print!("{}\n", &line.dimmed());
            }
        };

        match kind {
            VerboseErrorKind::Char(c) => {
                let error_msg = format!(
                    "expected '{}', found {}",
                    c,
                    substring.chars().next().unwrap()
                );
                println!("{}: {} {}", loc, "error:".red().bold(), error_msg);
                print_line(true);
                if column > 0 {
                    print!("{}", repeat(' ').take(column).collect::<String>());
                }
                println!("{}", "^".red().bold());
            }
            VerboseErrorKind::Context(s) => {
                let context_msg = format!("In {}", s);
                println!("{}: {}", loc, context_msg);
                print_line(false);
                if column > 0 {
                    print!("{}", repeat(' ').take(column).collect::<String>());
                }
                println!("{}", "^".blue().bold());
            }
            VerboseErrorKind::Nom(ek) => {
                println!("parsing error: {:#?}\n\n", ek);
            }
        }
    }
}
