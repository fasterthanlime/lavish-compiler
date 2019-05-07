use nom::{
    branch::alt,
    bytes::complete::{tag, take_until, take_while, take_while1},
    character::complete::char,
    combinator::{all_consuming, map, opt},
    error::{context, ParseError},
    multi::{many0, many1, separated_list},
    sequence::{delimited, preceded, terminated, tuple},
    IResult,
};

mod errors;
mod span;

use super::ast::*;
pub use errors::*;
pub use span::*;

pub fn module<'a, E: ParseError<Span>>(i: Span) -> IResult<Span, Module, E> {
    all_consuming(terminated(
        map(many0(preceded(sp, nsdecl)), |namespaces| {
            Module::new(namespaces)
        }),
        sp,
    ))(i)
}

fn spaced<'a, O, E: ParseError<Span>, F>(f: F) -> impl Fn(Span) -> IResult<Span, O, E>
where
    F: Fn(Span) -> IResult<Span, O, E>,
{
    terminated(preceded(sp, f), sp)
}

fn sp<'a, E: ParseError<Span>>(i: Span) -> IResult<Span, Span, E> {
    let chars = " \t\r\n";

    take_while(move |c| chars.contains(c))(i)
}

fn id<'a, E: ParseError<Span>>(i: Span) -> IResult<Span, Span, E> {
    let chars = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789_";

    take_while1(move |c| chars.contains(c))(i)
}

fn typ<'a, E: ParseError<Span>>(i: Span) -> IResult<Span, Span, E> {
    let chars = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789_<>";

    take_while1(move |c| chars.contains(c))(i)
}

fn loc<'a, E: ParseError<Span>>(i: Span) -> IResult<Span, Span, E> {
    let o = i.clone();
    Ok((i, o))
}

fn field<'a, E: ParseError<Span>>(i: Span) -> IResult<Span, Field, E> {
    let (i, comment) = opt(comment)(i)?;
    let (i, loc) = spaced(loc)(i)?;
    let (i, name) = spaced(id)(i)?;
    let ctx = spaced(context("field", preceded(spaced(char(':')), spaced(typ))));

    map(ctx, move |typ| Field {
        comment: comment.clone(),
        name: name.clone().into(),
        loc: loc.clone(),
        typ: typ.into(),
    })(i)
}

fn fields<'a, E: ParseError<Span>>(i: Span) -> IResult<Span, Vec<Field>, E> {
    terminated(
        separated_list(spaced(char(',')), field),
        opt(spaced(char(','))),
    )(i)
}

fn fnmod<'a, E: ParseError<Span>>(i: Span) -> IResult<Span, FunctionModifier, E> {
    alt((
        map(tag("server"), |_| FunctionModifier::Server),
        map(tag("client"), |_| FunctionModifier::Client),
    ))(i)
}

fn fnmods<'a, E: ParseError<Span>>(i: Span) -> IResult<Span, Vec<FunctionModifier>, E> {
    preceded(sp, separated_list(sp, fnmod))(i)
}

fn results<'a, E: ParseError<Span>>(i: Span) -> IResult<Span, Vec<Field>, E> {
    let (i, _) = spaced(tag("->"))(i)?;

    context(
        "result list",
        delimited(char('('), fields, preceded(sp, char(')'))),
    )(i)
}

fn fndecl<'a, E: ParseError<Span>>(i: Span) -> IResult<Span, FunctionDecl, E> {
    let (i, comment) = opt(comment)(i)?;
    let (i, modifiers) = fnmods(i)?;
    let (i, loc) = spaced(loc)(i)?;
    let (i, _) = spaced(tag("fn"))(i)?;

    context(
        "function declaration",
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
                loc: loc.clone(),
                comment: comment.clone(),
                modifiers: modifiers.clone(),
                name: name.into(),
                params,
                results: results.unwrap_or_default(),
            },
        ),
    )(i)
}

fn structdecl<'a, E: ParseError<Span>>(i: Span) -> IResult<Span, StructDecl, E> {
    let (i, comment) = opt(comment)(i)?;
    let (i, loc) = spaced(loc)(i)?;
    let (i, _) = preceded(sp, tag("struct"))(i)?;

    context(
        "struct",
        map(
            tuple((
                preceded(sp, id),
                preceded(sp, delimited(char('{'), sp, char('}'))),
            )),
            move |(name, _)| StructDecl {
                loc: loc.clone(),
                comment: comment.clone(),
                name: name.into(),
                fields: Vec::new(),
            },
        ),
    )(i)
}

fn comment_line<'a, E: ParseError<Span>>(i: Span) -> IResult<Span, Span, E> {
    preceded(sp, preceded(tag("//"), preceded(sp, take_until("\n"))))(i)
}

fn comment<'a, E: ParseError<Span>>(i: Span) -> IResult<Span, Comment, E> {
    map(many1(comment_line), |lines| Comment {
        lines: lines.iter().map(|x| x.clone().into()).collect(),
    })(i)
}

fn nsitem<'a, E: ParseError<Span>>(i: Span) -> IResult<Span, NamespaceItem, E> {
    alt((
        map(fndecl, |i| NamespaceItem::Function(i)),
        map(structdecl, |i| NamespaceItem::Struct(i)),
        map(nsdecl, |i| NamespaceItem::Namespace(i)),
    ))(i)
}

fn nsbody<'a, E: ParseError<Span>>(i: Span) -> IResult<Span, Vec<NamespaceItem>, E> {
    many0(preceded(sp, nsitem))(i)
}

fn nsdecl<'a, E: ParseError<Span>>(i: Span) -> IResult<Span, NamespaceDecl, E> {
    let (i, comment) = opt(comment)(i)?;
    let (i, loc) = spaced(loc)(i)?;
    let (i, _) = terminated(preceded(sp, tag("namespace")), sp)(i)?;

    context(
        "namespace",
        map(
            tuple((
                spaced(id),
                delimited(spaced(char('{')), nsbody, spaced(char('}'))),
            )),
            move |(name, items)| {
                NamespaceDecl::new(name.into(), loc.clone(), comment.clone(), items)
            },
        ),
    )(i)
}
