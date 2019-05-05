use nom::{
    branch::alt,
    bytes::complete::{tag, take_while},
    character::complete::{alphanumeric1, char},
    combinator::map,
    error::{context, ParseError, VerboseError, VerboseErrorKind},
    multi::many0,
    sequence::{delimited, preceded, tuple},
    IResult, Offset,
};
use std::iter::repeat;

use super::ast::*;

pub fn root<'a, E: ParseError<&'a str>>(i: &'a str) -> IResult<&'a str, Vec<NamespaceDecl>, E> {
    many0(preceded(sp, nsdecl))(i)
}

fn sp<'a, E: ParseError<&'a str>>(i: &'a str) -> IResult<&'a str, &'a str, E> {
    let chars = " \t\r\n";

    take_while(move |c| chars.contains(c))(i)
}

fn id<'a, E: ParseError<&'a str>>(i: &'a str) -> IResult<&'a str, &'a str, E> {
    let chars = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789_";

    take_while(move |c| chars.contains(c))(i)
}

fn fndecl<'a, E: ParseError<&'a str>>(i: &'a str) -> IResult<&'a str, FunctionDecl, E> {
    let (i, _) = tag("fn")(i)?;

    context(
        "fn",
        map(
            tuple((
                preceded(sp, id),
                preceded(sp, delimited(char('('), sp, char(')'))),
            )),
            |(name, _)| FunctionDecl { name: name.into() },
        ),
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
    let (i, _) = tag("namespace")(i)?;

    context(
        "namespace",
        map(
            tuple((
                preceded(sp, id),
                delimited(preceded(sp, char('{')), nsbody, preceded(sp, char('}'))),
            )),
            |(name, items)| NamespaceDecl::new(name, items),
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
