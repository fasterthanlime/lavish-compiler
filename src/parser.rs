use nom::{
    branch::alt,
    bytes::complete::{tag, take_until, take_while},
    character::complete::char,
    combinator::{all_consuming, map, opt},
    error::{context, ParseError, VerboseError, VerboseErrorKind},
    multi::{many0, many1, separated_list},
    sequence::{delimited, preceded, separated_pair, terminated, tuple},
    IResult, Offset,
};
use std::iter::repeat;

use super::ast::*;

pub fn root<'a, E: ParseError<&'a str>>(i: &'a str) -> IResult<&'a str, Vec<NamespaceDecl>, E> {
    all_consuming(terminated(many0(preceded(sp, nsdecl)), sp))(i)
}

fn sp<'a, E: ParseError<&'a str>>(i: &'a str) -> IResult<&'a str, &'a str, E> {
    let chars = " \t\r\n";

    take_while(move |c| chars.contains(c))(i)
}

fn id<'a, E: ParseError<&'a str>>(i: &'a str) -> IResult<&'a str, &'a str, E> {
    let chars = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789_";

    take_while(move |c| chars.contains(c))(i)
}

fn field<'a, E: ParseError<&'a str>>(i: &'a str) -> IResult<&'a str, Field, E> {
    map(
        tuple((
            opt(comment),
            separated_pair(preceded(sp, id), preceded(sp, char(':')), preceded(sp, id)),
        )),
        |(comment, (name, typ))| Field {
            comment,
            name: name.into(),
            typ: typ.into(),
        },
    )(i)
}

fn fields<'a, E: ParseError<&'a str>>(i: &'a str) -> IResult<&'a str, Vec<Field>, E> {
    separated_list(preceded(sp, char(',')), field)(i)
}

fn fndecl<'a, E: ParseError<&'a str>>(i: &'a str) -> IResult<&'a str, FunctionDecl, E> {
    map(
        tuple((
            opt(preceded(sp, comment)),
            preceded(sp, preceded(tag("fn"), preceded(sp, id))),
            preceded(
                sp,
                delimited(char('('), context("function parameters", fields), char(')')),
            ),
            opt(preceded(
                sp,
                preceded(
                    tag("->"),
                    preceded(
                        sp,
                        delimited(char('('), context("function results", fields), char(')')),
                    ),
                ),
            )),
        )),
        |(comment, name, params, results)| FunctionDecl {
            comment,
            name: name.into(),
            params,
            results: results.unwrap_or_default(),
        },
    )(i)
}

fn structdecl<'a, E: ParseError<&'a str>>(i: &'a str) -> IResult<&'a str, StructDecl, E> {
    let (i, _) = tag("struct")(i)?;

    context(
        "struct",
        map(
            tuple((
                preceded(sp, id),
                preceded(sp, delimited(char('{'), sp, char('}'))),
            )),
            |(name, _)| StructDecl { name: name.into() },
        ),
    )(i)
}

fn comment_line<'a, E: ParseError<&'a str>>(i: &'a str) -> IResult<&'a str, &'a str, E> {
    preceded(sp, preceded(tag("//"), preceded(sp, take_until("\n"))))(i)
}

fn comment<'a, E: ParseError<&'a str>>(i: &'a str) -> IResult<&'a str, Comment, E> {
    map(many1(comment_line), |lines| Comment {
        lines: lines.iter().map(|&x| x.into()).collect(),
    })(i)
}

fn nsitem<'a, E: ParseError<&'a str>>(i: &'a str) -> IResult<&'a str, NamespaceItem, E> {
    alt((
        map(fndecl, |i| NamespaceItem::Function(i)),
        map(structdecl, |i| NamespaceItem::Struct(i)),
        map(nsdecl, |i| NamespaceItem::Namespace(i)),
    ))(i)
}

fn nsbody<'a, E: ParseError<&'a str>>(i: &'a str) -> IResult<&'a str, Vec<NamespaceItem>, E> {
    many0(preceded(sp, nsitem))(i)
}

fn nsdecl<'a, E: ParseError<&'a str>>(i: &'a str) -> IResult<&'a str, NamespaceDecl, E> {
    let (i, comment) = opt(comment)(i)?;
    let (i, _) = preceded(sp, tag("namespace"))(i)?;

    context(
        "namespace",
        map(
            tuple((
                preceded(sp, id),
                delimited(
                    preceded(sp, char('{')),
                    context("namespace", nsbody),
                    preceded(sp, char('}')),
                ),
            )),
            move |(name, items)| NamespaceDecl::new(name, comment.clone(), items),
        ),
    )(i)
}

pub fn convert_error(input: &str, e: VerboseError<&str>) -> String {
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
