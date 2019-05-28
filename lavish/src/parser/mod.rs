use nom::{
    branch::alt,
    bytes::complete::{tag, take_until, take_while, take_while1},
    character::complete::char,
    combinator::{all_consuming, cut, map, opt},
    error::{context, ParseError},
    multi::{many0, many1, separated_list},
    sequence::{delimited, preceded, separated_pair, terminated, tuple},
    IResult, InputTake,
};

mod errors;
mod span;

use super::ast::*;
pub use errors::*;
pub use span::*;

use std::collections::HashSet;
use std::iter::FromIterator;

/// Parses an entire lavish schema
pub fn schema<E: ParseError<Span>>(i: Span) -> IResult<Span, Schema, E> {
    let (i, loc) = loc(i)?;

    all_consuming(terminated(
        map(tuple((imports, spaced(nsbody))), move |(imports, body)| {
            Schema::new(loc.clone(), imports, body)
        }),
        spaced(many0(spaced(comment_line))),
    ))(i)
}

/// Parses a `lavish-rules` files.
pub fn rules<E: ParseError<Span>>(i: Span) -> IResult<Span, Rules, E> {
    let (i, loc) = loc(i)?;

    all_consuming(terminated(
        map(tuple((target, spaced(builds))), move |(target, builds)| {
            Rules::new(loc.clone(), target, builds)
        }),
        spaced(many0(spaced(comment_line))),
    ))(i)
}

/// In rules file: `target {rust,go,typescript}`, with an optional body
pub fn target<E: ParseError<Span>>(i: Span) -> IResult<Span, Target, E> {
    let (i, _) = spaced(tag("target"))(i)?;

    context(
        "target directive",
        cut(alt((
            map(rust_target, Target::Rust),
            map(go_target, Target::Go),
            map(ts_target, Target::TypeScript),
        ))),
    )(i)
}

/// In rules: `target rust`
pub fn rust_target<E: ParseError<Span>>(i: Span) -> IResult<Span, RustTarget, E> {
    let (i, _) = spaced(tag("rust"))(i)?;

    Ok((i, RustTarget {}))
}

/// In rules: `target go`
pub fn go_target<E: ParseError<Span>>(i: Span) -> IResult<Span, GoTarget, E> {
    let (i, _) = spaced(tag("go"))(i)?;

    Ok((i, GoTarget {}))
}

/// In rules: `target typescript`
pub fn ts_target<E: ParseError<Span>>(i: Span) -> IResult<Span, TypeScriptTarget, E> {
    let (i, _) = spaced(tag("typescript"))(i)?;

    Ok((i, TypeScriptTarget {}))
}

/// In rules: list of build directives
pub fn builds<E: ParseError<Span>>(i: Span) -> IResult<Span, Vec<Build>, E> {
    many0(spaced(build))(i)
}

/// In rules: `build X [from Y]`
pub fn build<E: ParseError<Span>>(i: Span) -> IResult<Span, Build, E> {
    let (i, _) = many0(spaced(comment_line))(i)?;
    let (i, _) = spaced(tag("build"))(i)?;

    context(
        "build directive",
        cut(map(
            tuple((spaced(id), spaced(opt(from)))),
            |(name, from)| Build { name, from },
        )),
    )(i)
}

/// From directive, used for `build` (rules) and `import` (schemas)
pub fn from<E: ParseError<Span>>(i: Span) -> IResult<Span, FromDirective, E> {
    let (i, _) = loc(i)?;
    let (i, _) = spaced(tag("from"))(i)?;

    context(
        "from directive",
        cut(map(spaced(stringlit), |path| FromDirective { path })),
    )(i)
}

/// In schema: 0+ import directives
pub fn imports<E: ParseError<Span>>(i: Span) -> IResult<Span, Vec<Import>, E> {
    many0(spaced(import))(i)
}

/// In schema: single import directive
pub fn import<E: ParseError<Span>>(i: Span) -> IResult<Span, Import, E> {
    let (i, _) = spaced(tag("import"))(i)?;

    context(
        "import",
        cut(map(
            tuple((spaced(id), spaced(opt(from)))),
            |(name, from)| Import { name, from },
        )),
    )(i)
}

/// f, but skip whitespace before and after (including newlines)
fn spaced<O, E: ParseError<Span>, F>(f: F) -> impl Fn(Span) -> IResult<Span, O, E>
where
    F: Fn(Span) -> IResult<Span, O, E>,
{
    terminated(preceded(sp, f), sp)
}

/// All whitespace (including newlines)
fn sp<E: ParseError<Span>>(i: Span) -> IResult<Span, Span, E> {
    let chars = " \t\r\n";

    take_while(move |c| chars.contains(c))(i)
}

/// Whitespace excluding newlines
fn linesp<E: ParseError<Span>>(i: Span) -> IResult<Span, Span, E> {
    let chars = " \t";

    take_while(move |c| chars.contains(c))(i)
}

/// Identifier
fn id<E: ParseError<Span>>(i: Span) -> IResult<Span, Identifier, E> {
    let chars = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789_";

    map(take_while1(move |c| chars.contains(c)), |span: Span| {
        let text = span.clone().into();
        Identifier { span, text }
    })(i)
}

/// String literal, without escapes for now
pub fn stringlit<E: ParseError<Span>>(i: Span) -> IResult<Span, StringLiteral, E> {
    // TODO: use escaped_transform instead
    let (i, loc) = loc(i)?;

    let chars = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789_-./";

    map(
        delimited(
            char('"'),
            take_while1(move |c| chars.contains(c)),
            char('"'),
        ),
        move |span: Span| {
            let value = span.clone().into();
            StringLiteral {
                loc: loc.clone(),
                value,
            }
        },
    )(i)
}

/// Builtin lavish types
fn basetyp<E: ParseError<Span>>(i: Span) -> IResult<Span, Type, E> {
    map(
        spaced(alt((
            map(tag("bool"), |span| (span, BaseType::Bool)),
            map(tag("int32"), |span| (span, BaseType::Int32)),
            map(tag("int64"), |span| (span, BaseType::Int64)),
            map(tag("uint32"), |span| (span, BaseType::UInt32)),
            map(tag("uint64"), |span| (span, BaseType::UInt64)),
            map(tag("float32"), |span| (span, BaseType::Float32)),
            map(tag("float64"), |span| (span, BaseType::Float64)),
            map(tag("string"), |span| (span, BaseType::String)),
            map(tag("bytes"), |span| (span, BaseType::Bytes)),
            map(tag("timestamp"), |span| (span, BaseType::Timestamp)),
        ))),
        |(span, basetyp)| Type {
            span,
            kind: TypeKind::Base(basetyp),
        },
    )(i)
}

/// Array type: Array<T>
fn arraytyp<E: ParseError<Span>>(i: Span) -> IResult<Span, Type, E> {
    let span = i.clone();
    map(
        preceded(
            terminated(spaced(tag("Array")), spaced(char('<'))),
            terminated(typ, spaced(char('>'))),
        ),
        move |t| Type {
            span: span.clone(),
            kind: TypeKind::Array(ArrayType { inner: Box::new(t) }),
        },
    )(i)
}

/// Option type: Option<T>
fn optiontyp<E: ParseError<Span>>(i: Span) -> IResult<Span, Type, E> {
    let span = i.clone();
    map(
        preceded(
            terminated(spaced(tag("Option")), spaced(char('<'))),
            terminated(typ, spaced(char('>'))),
        ),
        move |t| Type {
            span: span.clone(),
            kind: TypeKind::Option(OptionType { inner: Box::new(t) }),
        },
    )(i)
}

/// Map type: Map<K, V>
fn maptyp<E: ParseError<Span>>(i: Span) -> IResult<Span, Type, E> {
    let span = i.clone();
    map(
        preceded(
            terminated(spaced(tag("Map")), spaced(char('<'))),
            terminated(
                separated_pair(spaced(typ), char(','), spaced(typ)),
                spaced(char('>')),
            ),
        ),
        move |(k, v)| Type {
            span: span.clone(),
            kind: TypeKind::Map(MapType {
                keys: Box::new(k),
                values: Box::new(v),
            }),
        },
    )(i)
}

/// User type: foo.bar.Baz
fn usertyp<E: ParseError<Span>>(i: Span) -> IResult<Span, Type, E> {
    let chars = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789_.";

    map(take_while1(move |c| chars.contains(c)), |span: Span| Type {
        span,
        kind: TypeKind::User,
    })(i)
}

// Any valid lavish type
fn typ<E: ParseError<Span>>(i: Span) -> IResult<Span, Type, E> {
    alt((maptyp, arraytyp, optiontyp, basetyp, usertyp))(i)
}

// Consumes nothing, returns a span for location information
fn loc<E: ParseError<Span>>(i: Span) -> IResult<Span, Span, E> {
    let o = i.take(0);
    Ok((i, o))
}

// Field declaration: `name: type`, prefixed by optional comment
fn field<E: ParseError<Span>>(i: Span) -> IResult<Span, Field, E> {
    let (i, comment) = opt(comment)(i)?;
    let (i, loc) = spaced(loc)(i)?;
    let (i, name) = spaced(id)(i)?;
    let ctx = spaced(context(
        "field",
        cut(preceded(spaced(char(':')), spaced(typ))),
    ));

    map(ctx, move |typ| Field {
        comment: comment.clone(),
        name: name.clone(),
        loc: loc.clone(),
        typ,
    })(i)
}

// Field list: field declarations separated by commas
fn fields<E: ParseError<Span>>(i: Span) -> IResult<Span, Vec<Field>, E> {
    terminated(
        separated_list(spaced(char(',')), field),
        opt(spaced(char(','))),
    )(i)
}

// Function modifiers (server, client)
fn fnmod<E: ParseError<Span>>(i: Span) -> IResult<Span, FunctionModifier, E> {
    alt((
        map(tag("server"), |_| FunctionModifier::Server),
        map(tag("client"), |_| FunctionModifier::Client),
    ))(i)
}

// Any number of function modifiers
fn fnmods<E: ParseError<Span>>(i: Span) -> IResult<Span, Vec<FunctionModifier>, E> {
    preceded(sp, separated_list(sp, fnmod))(i)
}

// Results, in the context of a function declaration: `-> (fields)`
fn results<E: ParseError<Span>>(i: Span) -> IResult<Span, Vec<Field>, E> {
    let (i, _) = spaced(tag("->"))(i)?;

    context(
        "result list",
        cut(delimited(char('('), fields, preceded(sp, char(')')))),
    )(i)
}

// Function declaration
fn fndecl<E: ParseError<Span>>(i: Span) -> IResult<Span, FunctionDecl, E> {
    let (i, comment) = opt(comment)(i)?;
    let (i, modifiers) = fnmods(i)?;
    let (i, _) = spaced(tag("fn"))(i)?;
    let (i, loc) = spaced(loc)(i)?;

    context(
        "function declaration",
        cut(map(
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
                opt(fnbody),
            )),
            move |(name, params, results, body)| FunctionDecl {
                loc: loc.clone(),
                comment: comment.clone(),
                modifiers: HashSet::from_iter(modifiers.iter().cloned()),
                name: name.clone(),
                params,
                results: results.unwrap_or_default(),
                body,
            },
        )),
    )(i)
}

// Function body (nested functions)
fn fnbody<E: ParseError<Span>>(i: Span) -> IResult<Span, NamespaceBody, E> {
    let (i, _) = spaced(char('{'))(i)?;

    context("function body", cut(terminated(nsbody, spaced(char('}')))))(i)
}

// Notification declaration: like function, but no results and no body
fn notifdecl<E: ParseError<Span>>(i: Span) -> IResult<Span, FunctionDecl, E> {
    let (i, comment) = opt(comment)(i)?;
    let (i, modifiers) = fnmods(i)?;
    let (i, _) = spaced(tag("nf"))(i)?;
    let (i, loc) = spaced(loc)(i)?;

    context(
        "notification declaration",
        cut(map(
            tuple((
                preceded(sp, id),
                preceded(
                    sp,
                    context(
                        "parameter list",
                        delimited(char('('), fields, preceded(sp, char(')'))),
                    ),
                ),
            )),
            move |(name, params)| FunctionDecl {
                loc: loc.clone(),
                comment: comment.clone(),
                modifiers: {
                    let mut hs = HashSet::from_iter(modifiers.iter().cloned());
                    hs.insert(FunctionModifier::Notification);
                    hs
                },
                name: name.clone(),
                params,
                results: Vec::new(),
                body: None,
            },
        )),
    )(i)
}

// Struct declaration
fn structdecl<E: ParseError<Span>>(i: Span) -> IResult<Span, StructDecl, E> {
    let (i, comment) = opt(comment)(i)?;
    let (i, _) = preceded(sp, tag("struct"))(i)?;
    let (i, loc) = spaced(loc)(i)?;

    context(
        "struct declaration",
        cut(map(
            tuple((
                preceded(sp, id),
                preceded(sp, delimited(char('{'), fields, preceded(sp, char('}')))),
            )),
            move |(name, fields)| StructDecl {
                loc: loc.clone(),
                comment: comment.clone(),
                name: name.clone(),
                fields,
            },
        )),
    )(i)
}

// A single comment-line
fn comment_line<E: ParseError<Span>>(i: Span) -> IResult<Span, Span, E> {
    preceded(sp, preceded(tag("//"), preceded(linesp, take_until("\n"))))(i)
}

// A comment block, made of 1+ comment lines. Use with opt
fn comment<E: ParseError<Span>>(i: Span) -> IResult<Span, Comment, E> {
    map(many1(comment_line), |lines| Comment {
        lines: lines.iter().map(|x| x.clone().into()).collect(),
    })(i)
}

// Any namespace item, will be Some() if we found something other
// then a comment line
fn nsitem<E: ParseError<Span>>(i: Span) -> IResult<Span, Option<NamespaceItem>, E> {
    alt((
        map(
            alt((
                map(fndecl, NamespaceItem::Function),
                map(notifdecl, NamespaceItem::Function),
                map(structdecl, NamespaceItem::Struct),
                map(nsdecl, NamespaceItem::Namespace),
            )),
            Some,
        ),
        map(comment_line, |_| None),
    ))(i)
}

// A namespace body, but also a function body.
fn nsbody<E: ParseError<Span>>(i: Span) -> IResult<Span, NamespaceBody, E> {
    terminated(
        map(many0(spaced(nsitem)), |mut x| {
            NamespaceBody::new(x.drain(..).filter_map(|x| x).collect())
        }),
        spaced(many0(comment_line)),
    )(i)
}

// A namespace declaration: `namespace X { nsbody }`
fn nsdecl<E: ParseError<Span>>(i: Span) -> IResult<Span, NamespaceDecl, E> {
    let (i, comment) = opt(comment)(i)?;
    let (i, _) = terminated(preceded(sp, tag("namespace")), sp)(i)?;
    let (i, loc) = spaced(loc)(i)?;

    context(
        "namespace declaration",
        cut(map(
            tuple((
                spaced(id),
                delimited(spaced(char('{')), nsbody, spaced(char('}'))),
            )),
            move |(name, items)| NamespaceDecl::new(name, loc.clone(), comment.clone(), items),
        )),
    )(i)
}
