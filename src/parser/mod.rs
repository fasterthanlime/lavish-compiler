use nom::{
    branch::alt,
    bytes::complete::{tag, take_until, take_while},
    character::complete::char,
    combinator::{all_consuming, map, opt},
    error::{context, ParseError},
    multi::{many0, many1, separated_list},
    sequence::{delimited, preceded, separated_pair, terminated, tuple},
    IResult,
};

mod errors;
use super::ast::*;
pub use errors::*;

pub fn root<'a, E: ParseError<&'a str>>(i: &'a str) -> IResult<&'a str, Vec<NamespaceDecl>, E> {
    all_consuming(terminated(many0(preceded(sp, nsdecl)), sp))(i)
}

fn spaced<'a, O, E: ParseError<&'a str>, F>(f: F) -> impl Fn(&'a str) -> IResult<&'a str, O, E>
where
    F: Fn(&'a str) -> IResult<&'a str, O, E>,
{
    terminated(preceded(sp, f), sp)
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
            separated_pair(spaced(id), spaced(char(':')), spaced(id)),
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

fn fnmod<'a, E: ParseError<&'a str>>(i: &'a str) -> IResult<&'a str, FunctionModifier, E> {
    alt((
        map(tag("server"), |_| FunctionModifier::Server),
        map(tag("client"), |_| FunctionModifier::Client),
    ))(i)
}

fn fnmods<'a, E: ParseError<&'a str>>(i: &'a str) -> IResult<&'a str, Vec<FunctionModifier>, E> {
    preceded(sp, separated_list(sp, fnmod))(i)
}

fn results<'a, E: ParseError<&'a str>>(i: &'a str) -> IResult<&'a str, Vec<Field>, E> {
    let (i, _) = spaced(tag("->"))(i)?;

    context(
        "result list",
        delimited(char('('), fields, preceded(sp, char(')'))),
    )(i)
}

fn fndecl<'a, E: ParseError<&'a str>>(i: &'a str) -> IResult<&'a str, FunctionDecl, E> {
    let (i, comment) = opt(comment)(i)?;
    let (i, modifiers) = fnmods(i)?;
    let (i, _) = spaced(tag("fn"))(i)?;

    context(
        "fn",
        map(
            tuple((
                preceded(sp, id),
                preceded(
                    sp,
                    context(
                        "parameter list",
                        delimited(char('('), fields, preceded(sp, char(')'))),
                    ),
                ),
                opt(results),
            )),
            move |(name, params, results)| FunctionDecl {
                comment: comment.clone(),
                modifiers: modifiers.clone(),
                name: name.into(),
                params,
                results: results.unwrap_or_default(),
            },
        ),
    )(i)
}

fn structdecl<'a, E: ParseError<&'a str>>(i: &'a str) -> IResult<&'a str, StructDecl, E> {
    let (i, comment) = opt(comment)(i)?;
    let (i, _) = preceded(sp, tag("struct"))(i)?;

    context(
        "struct",
        map(
            tuple((
                preceded(sp, id),
                preceded(sp, delimited(char('{'), sp, char('}'))),
            )),
            move |(name, _)| StructDecl {
                comment: comment.clone(),
                name: name.into(),
            },
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
    let (i, _) = terminated(preceded(sp, tag("namespace")), sp)(i)?;

    context(
        "namespace",
        map(
            tuple((
                spaced(id),
                delimited(spaced(char('{')), nsbody, spaced(char('}'))),
            )),
            move |(name, items)| NamespaceDecl::new(name, comment.clone(), items),
        ),
    )(i)
}
