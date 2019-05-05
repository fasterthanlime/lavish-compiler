use colored::*;
use nom::{
    error::{VerboseError, VerboseErrorKind},
    Offset,
};
use std::iter::repeat;

pub fn print_errors(input_name: &str, input: &str, e: VerboseError<&str>) {
    let input_name = input_name.replace("./", "");

    // FIXME: this only works with <LF> or <CR> line endings, not <CRLF>
    let lines: Vec<_> = input.lines().map(String::from).collect();
    // println!(
    //     "lines:\n{}",
    //     lines
    //         .iter()
    //         .enumerate()
    //         .map(|(i, line)| format!("{:5} | {}", i, line))
    //         .collect::<Vec<String>>()
    //         .join("\n")
    // );

    for (_, (substring, kind)) in e.errors.iter().enumerate() {
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

        let loc = format!("{}:{}:{}", input_name, line, column);
        let loc = loc.bold();

        match kind {
            VerboseErrorKind::Char(c) => {
                let error_msg = format!(
                    "expected '{}', found {}",
                    c,
                    substring.chars().next().unwrap()
                );
                println!("{}: {} {}", loc, "error:".red().bold(), error_msg);
                println!("{}", lines[line]);
                if column > 0 {
                    print!("{}", repeat(' ').take(column).collect::<String>());
                }
                println!("{}", "^".red().bold());
            }
            VerboseErrorKind::Context(s) => {
                let context_msg = format!("occured in {}", s);
                println!("{}: {} {}", loc, "note:".blue().bold(), context_msg);
                println!("{}", lines[line]);
                if column > 0 {
                    print!("{}", repeat(' ').take(column).collect::<String>());
                }
                println!("{}\n", "^".blue().bold());
            }
            VerboseErrorKind::Nom(ek) => {
                println!("parsing error: {:#?}\n\n", ek);
            }
        }
    }
}
