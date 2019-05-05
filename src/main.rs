use nom::{
    branch::alt,
    bytes::complete::{tag, take_while},
    character::complete::{alphanumeric1, char},
    combinator::map,
    error::{context, ParseError, VerboseError, VerboseErrorKind},
    multi::many0,
    sequence::{delimited, preceded, tuple},
    Err, IResult, Offset,
};
use std::iter::repeat;

fn main() {
    let data = "namespace butlerd {
        struct
    }
    namespace pingpong {
        fn ping
    }";
    match root::<VerboseError<&str>>(data) {
        Err(Err::Error(e)) | Err(Err::Failure(e)) => {
            println!(
                "verbose errors - `root::<VerboseError>(data)`:\n{}",
                convert_error(data, e)
            );
        }
        Ok(res) => println!("Parsed: {:#?}", res),
        _ => println!("Something else happened :o"),
    }
}

#[derive(Debug)]
struct Namespace {
    name: String,
    items: Vec<NamespaceItem>,
}

#[derive(Debug)]
enum NamespaceItem {
    Fn { name: String },
    Struct,
}

fn root<'a, E: ParseError<&'a str>>(i: &'a str) -> IResult<&'a str, Vec<Namespace>, E> {
    many0(preceded(sp, ns))(i)
}

fn sp<'a, E: ParseError<&'a str>>(i: &'a str) -> IResult<&'a str, &'a str, E> {
    let chars = " \t\r\n";

    take_while(move |c| chars.contains(c))(i)
}

fn id<'a, E: ParseError<&'a str>>(i: &'a str) -> IResult<&'a str, &'a str, E> {
    alphanumeric1(i)
}

fn nsbody<'a, E: ParseError<&'a str>>(i: &'a str) -> IResult<&'a str, Vec<NamespaceItem>, E> {
    many0(preceded(
        sp,
        alt((
            |i| {
                preceded(tag("fn"), preceded(sp, id))(i).map(|(i, name)| {
                    (
                        i,
                        NamespaceItem::Fn {
                            name: String::from(name),
                        },
                    )
                })
            },
            |i| tag("struct")(i).map(|(i, _)| (i, NamespaceItem::Struct {})),
        )),
    ))(i)
}

fn ns<'a, E: ParseError<&'a str>>(i: &'a str) -> IResult<&'a str, Namespace, E> {
    let (i, _) = tag("namespace")(i)?;

    context(
        "namespace",
        map(
            tuple((
                preceded(sp, id),
                delimited(preceded(sp, char('{')), nsbody, preceded(sp, char('}'))),
            )),
            |(name, items)| Namespace {
                name: String::from(name),
                items,
            },
        ),
    )(i)
}

fn convert_error(input: &str, e: VerboseError<&str>) -> String {
    let lines: Vec<_> = input.lines().map(String::from).collect();
    //println!("lines: {:#?}", lines);

    let mut result = String::new();

    for (i, (substring, kind)) in e.errors.iter().enumerate() {
        let mut offset = input.offset(substring);

        let mut line = 0;
        let mut column = 0;

        for (j, l) in lines.iter().enumerate() {
            if offset <= l.len() {
                line = j;
                column = offset;
                break;
            } else {
                offset = offset - l.len();
            }
        }

        match kind {
            VerboseErrorKind::Char(c) => {
                result += &format!("{}: at line {}:\n", i, line);
                result += &lines[line];
                result += "\n";
                if column > 0 {
                    result += &repeat(' ').take(column - 1).collect::<String>();
                }
                result += "^\n";
                result += &format!(
                    "expected '{}', found {}\n\n",
                    c,
                    substring.chars().next().unwrap()
                );
            }
            VerboseErrorKind::Context(s) => {
                result += &format!("{}: at line {}, in {}:\n", i, line, s);
                result += &lines[line];
                result += "\n";
                if column > 0 {
                    result += &repeat(' ').take(column - 1).collect::<String>();
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
