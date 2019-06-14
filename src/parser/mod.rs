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

    context(
        "rust target",
        cut(map(
            opt(delimited(
                spaced(char('{')),
                rust_target_body,
                spaced(char('}')),
            )),
            move |items| RustTarget::new(items.unwrap_or_default()),
        )),
    )(i)
}

pub fn rust_target_body<E: ParseError<Span>>(i: Span) -> IResult<Span, Vec<RustTargetItem>, E> {
    many0(spaced(rust_target_item))(i)
}

pub fn rust_target_item<E: ParseError<Span>>(i: Span) -> IResult<Span, RustTargetItem, E> {
    map(rust_target_wrapper, RustTargetItem::Wrapper)(i)
}

pub fn rust_target_wrapper<E: ParseError<Span>>(i: Span) -> IResult<Span, RustTargetWrapper, E> {
    let (i, _) = spaced(tag("wrapper"))(i)?;

    context(
        "rust target wrapper",
        cut(preceded(
            spaced(char('=')),
            alt((
                map(tag("none"), |_| RustTargetWrapper::None),
                map(tag("mod"), |_| RustTargetWrapper::Mod),
                map(tag("lib"), |_| RustTargetWrapper::Lib),
            )),
        )),
    )(i)
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
            map(tag("i8"), |span| (span, BaseType::I8)),
            map(tag("i16"), |span| (span, BaseType::I16)),
            map(tag("i32"), |span| (span, BaseType::I32)),
            map(tag("i64"), |span| (span, BaseType::I64)),
            map(tag("u8"), |span| (span, BaseType::U8)),
            map(tag("u16"), |span| (span, BaseType::U16)),
            map(tag("u32"), |span| (span, BaseType::U32)),
            map(tag("u64"), |span| (span, BaseType::U64)),
            map(tag("f32"), |span| (span, BaseType::F32)),
            map(tag("f64"), |span| (span, BaseType::F64)),
            map(tag("string"), |span| (span, BaseType::String)),
            map(tag("data"), |span| (span, BaseType::Data)),
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
            terminated(spaced(tag("array")), spaced(char('<'))),
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
            terminated(spaced(tag("option")), spaced(char('<'))),
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
            terminated(spaced(tag("map")), spaced(char('<'))),
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
fn side<E: ParseError<Span>>(i: Span) -> IResult<Span, Side, E> {
    alt((
        map(tag("server"), |_| Side::Server),
        map(tag("client"), |_| Side::Client),
    ))(i)
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
    let (i, side) = side(i)?;
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
                side,
                kind: Kind::Request,
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
    let (i, side) = side(i)?;
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
                kind: Kind::Notification,
                side,
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

#[cfg(test)]
mod tests {
    use super::errors::*;
    use strip_ansi_escapes;

    macro_rules! parse_passing {
        ($parse: ident, $name: ident) => {
            #[test]
            fn $name() -> Result<(), Error> {
                $parse(Source::from_string(include_str!(concat!(
                    "tests/",
                    stringify!($name),
                    ".lavish"
                ))))?;
                Ok(())
            }
        };
    }

    macro_rules! parse_failing {
        ($parse: ident, $name: ident, $needle: expr) => {
            #[test]
            fn $name() -> Result<(), Error> {
                let res = $parse(Source::from_string(include_str!(concat!(
                    "tests/",
                    stringify!($name),
                    ".lavish"
                ))));
                match res {
                    Err(e) => {
                        let stripped = strip_ansi_escapes::strip(format!("{}", e))?;
                        let msg = String::from_utf8_lossy(&stripped);
                        if msg.contains($needle) {
                            Ok(())
                        } else {
                            Err(Error::UnexpectedSourceError(UnexpectedSourceError {
                                expected: $needle.into(),
                                actual: Some(Box::new(e)),
                            }))
                        }
                    }
                    Ok(_) => Err(Error::UnexpectedSourceError(UnexpectedSourceError {
                        expected: $needle.into(),
                        actual: None,
                    })),
                }
            }
        };
    }

    macro_rules! rules_passing {
        ($name: ident) => {
            parse_passing!(parse_rules, $name);
        };
    }

    macro_rules! rules_failing {
        ($name: ident, $needle: expr) => {
            parse_failing!(parse_rules, $name, $needle);
        };
    }

    macro_rules! schema_passing {
        ($name: ident) => {
            parse_passing!(parse_schema, $name);
        };
    }

    macro_rules! schema_failing {
        ($name: ident, $needle: expr) => {
            parse_failing!(parse_schema, $name, $needle);
        };
    }

    rules_passing!(target_rust);
    rules_passing!(target_go);
    rules_failing!(target_unknown, "parsing error: Tag");

    rules_passing!(build_local);
    rules_passing!(build_remote);

    schema_passing!(struct_cookie);
    schema_passing!(struct_comments);
    schema_failing!(struct_incomplete, "expected '}'");

    schema_passing!(fn_simple);
    schema_passing!(fn_namespaced);
    schema_passing!(fn_nested);
    schema_passing!(nf_simple);
}
