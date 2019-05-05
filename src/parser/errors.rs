use nom::{
    error::{VerboseError, VerboseErrorKind},
    Offset,
};
use std::iter::repeat;

pub fn convert_error(input: &str, e: VerboseError<&str>) -> String {
    // FIXME: this only works with <LF> or <CR> line endings, not <CRLF>
    let lines: Vec<_> = input.lines().map(String::from).collect();
    println!(
        "lines:\n{}",
        lines
            .iter()
            .enumerate()
            .map(|(i, line)| format!("{:5} | {}", i, line))
            .collect::<Vec<String>>()
            .join("\n")
    );

    let mut result = String::new();

    for (i, (substring, kind)) in e.errors.iter().enumerate() {
        let mut offset = input.offset(substring);
        result += &format!("offset: {:#?}\n", offset);

        let mut line = 0;
        let mut column = 0;

        for (j, l) in lines.iter().enumerate() {
            if offset <= l.len() {
                line = j;
                column = offset;
                break;
            } else {
                offset = offset - l.len() - 1;
            }
        }

        match kind {
            VerboseErrorKind::Char(c) => {
                result += &format!("{}: at [{}, {}]:\n", i, line, column);
                result += &lines[line];
                result += "\n";
                if column > 0 {
                    result += &repeat(' ').take(column).collect::<String>();
                }
                result += "^\n";
                result += &format!(
                    "expected '{}', found {}\n\n",
                    c,
                    substring.chars().next().unwrap()
                );
            }
            VerboseErrorKind::Context(s) => {
                result += &format!("{}: at [{}, {}], in {}:\n", i, line, column, s);
                result += &lines[line];
                result += "\n";
                if column > 0 {
                    result += &repeat(' ').take(column).collect::<String>();
                }
                result += "^\n\n";
            }
            VerboseErrorKind::Nom(ek) => {
                result += &format!("nom error {:#?}\n\n", ek);
            }
        }
    }

    result
}
