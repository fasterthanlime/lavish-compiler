use crate::codegen::output::*;
use std::fmt::{self, Display, Write};

pub trait WriteTo: Display {
    fn write_to(&self, s: &mut Scope) {
        write!(s, "{}", self).unwrap();
    }
}

impl<T> WriteTo for T where T: Display {}

pub struct Allow {
    items: Vec<&'static str>,
}

impl Allow {
    pub fn non_camel_case(mut self) -> Self {
        self.items.push("non_camel_case_types");
        self
    }

    pub fn unused(mut self) -> Self {
        self.items.push("unused");
        self
    }
}

impl Display for Allow {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "#[allow({items})]", items = self.items.join(", "))
    }
}

pub fn allow() -> Allow {
    Allow { items: Vec::new() }
}

pub fn serde_untagged() -> impl Display {
    "#[serde(untagged)]\n"
}

pub struct _Fn<'a> {
    kw_pub: bool,
    kw_async: bool,
    self_arg: Option<String>,
    params: Vec<String>,
    type_params: Vec<TypeParam>,
    name: String,
    ret: Option<String>,
    body: Option<Box<Fn(&mut Scope) + 'a>>,
}

impl<'a> _Fn<'a> {
    pub fn kw_pub(mut self) -> Self {
        self.kw_pub = true;
        self
    }

    pub fn kw_async(mut self) -> Self {
        self.kw_async = true;
        self
    }

    pub fn returns<D>(mut self, ret: D) -> Self
    where
        D: Display,
    {
        self.ret = Some(format!("{}", ret));
        self
    }

    pub fn body<F>(mut self, f: F) -> Self
    where
        F: Fn(&mut Scope) + 'a,
    {
        self.body = Some(Box::new(f));
        self
    }

    pub fn self_param<D>(mut self, self_arg: D) -> Self
    where
        D: Display,
    {
        self.self_arg = Some(format!("{}", self_arg));
        self
    }

    pub fn type_param<N>(mut self, name: N, constraint: Option<&str>) -> Self
    where
        N: Into<String>,
    {
        self.type_params.push(TypeParam {
            name: name.into(),
            constraint: constraint.map(|x| x.into()),
        });
        self
    }

    pub fn param<N>(mut self, name: N) -> Self
    where
        N: Into<String>,
    {
        self.params.push(name.into());
        self
    }
}

impl<'a> Display for _Fn<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        Scope::fmt(f, |s| {
            if self.kw_pub {
                s.write("pub ");
            }
            if self.kw_async {
                s.write("async ");
            }

            s.write("fn ").write(&self.name);
            s.in_list(Brackets::Angle, |l| {
                l.omit_empty();
                for tp in &self.type_params {
                    l.item(&tp.name);
                }
            });

            s.in_list(Brackets::Round, |l| {
                if let Some(self_param) = self.self_arg.as_ref() {
                    l.item(self_param);
                }
                for p in &self.params {
                    l.item(&p);
                }
            });

            if let Some(ret) = self.ret.as_ref() {
                s.write(" -> ").write(ret);
            }

            // TODO: write where clauses
            if let Some(body) = self.body.as_ref() {
                s.in_block(|s| {
                    body(s);
                });
            } else {
                s.write(";").lf();
            }
        })
    }
}

pub fn _fn<'a, N>(name: N) -> _Fn<'a>
where
    N: Into<String>,
{
    _Fn {
        kw_pub: false,
        kw_async: false,
        name: name.into(),
        params: Vec::new(),
        type_params: Vec::new(),
        self_arg: None,
        body: None,
        ret: None,
    }
}

pub struct _Impl<'a> {
    trt: String,
    name: String,
    type_params: Vec<TypeParam>,
    body: Option<Box<Fn(&mut Scope) + 'a>>,
}

impl<'a> _Impl<'a> {
    pub fn type_param(mut self, name: &str, constraint: Option<&str>) -> Self {
        self.type_params.push(TypeParam {
            name: name.into(),
            constraint: constraint.map(|x| x.into()),
        });
        self
    }

    pub fn type_params<P>(mut self, params: P) -> Self
    where
        P: AsRef<[TypeParam]>,
    {
        for param in params.as_ref() {
            self.type_params.push(param.clone());
        }
        self
    }

    pub fn body<F>(mut self, f: F) -> Self
    where
        F: Fn(&mut Scope) + 'a,
    {
        self.body = Some(Box::new(f));
        self
    }
}

pub fn _impl<'a, T, N>(trt: T, name: N) -> _Impl<'a>
where
    T: Into<String>,
    N: Into<String>,
{
    _Impl {
        trt: trt.into(),
        name: name.into(),
        type_params: Vec::new(),
        body: None,
    }
}

impl<'a> Display for _Impl<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        Scope::fmt(f, |s| {
            s.write("impl");
            s.in_list(Brackets::Angle, |l| {
                l.omit_empty();
                for tp in &self.type_params {
                    l.item(&tp.name);
                }
            });
            write!(s, " {trt} for {name}", trt = &self.trt, name = &self.name).unwrap();
            s.in_list(Brackets::Angle, |l| {
                l.omit_empty();
                for tp in &self.type_params {
                    match tp.constraint.as_ref() {
                        Some(constraint) => l.item(format!(
                            "{name}: {constraint}",
                            name = tp.name,
                            constraint = constraint
                        )),
                        None => l.item(&tp.name),
                    };
                }
            });

            s.in_block(|s| {
                if let Some(body) = self.body.as_ref() {
                    body(s);
                }
            });
        })
    }
}

#[derive(Clone)]
pub struct TypeParam {
    name: String,
    constraint: Option<String>,
}