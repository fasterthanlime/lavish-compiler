use crate::codegen::rust::prelude::*;

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
    self_param: Option<String>,
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

    pub fn self_param<D>(mut self, self_param: D) -> Self
    where
        D: Display,
    {
        self.self_param = Some(format!("{}", self_param));
        self
    }

    pub fn type_param<N, M>(mut self, name: N, bound: Option<M>) -> Self
    where
        N: Into<String>,
        M: Into<String>,
    {
        self.type_params.push(TypeParam {
            name: name.into(),
            bound: bound.map(|x| x.into()),
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

            s.write("fn ").write(&self.name);
            s.in_list(Brackets::Angle, |l| {
                l.omit_empty();
                for tp in &self.type_params {
                    l.item(&tp.name);
                }
            });

            s.in_list(Brackets::Round, |l| {
                if let Some(self_param) = self.self_param.as_ref() {
                    l.item(self_param);
                }
                for p in &self.params {
                    l.item(&p);
                }
            });

            if let Some(ret) = self.ret.as_ref() {
                s.write(" -> ").write(ret);
            }

            if self.type_params.iter().any(|tp| tp.bound.is_some()) {
                s.lf();
                s.write("where").lf();
                s.in_scope(|s| {
                    for tp in &self.type_params {
                        if let Some(bound) = tp.bound.as_ref() {
                            writeln!(s, "{name}: {bound},", name = tp.name, bound = bound).unwrap();
                        }
                    }
                });
            }

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
        name: name.into(),
        params: Vec::new(),
        type_params: Vec::new(),
        self_param: None,
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
    pub fn type_param(mut self, name: &str, bound: Option<&str>) -> Self {
        self.type_params.push(TypeParam {
            name: name.into(),
            bound: bound.map(|x| x.into()),
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
                    match tp.bound.as_ref() {
                        Some(bound) => {
                            l.item(format!("{name}: {bound}", name = tp.name, bound = bound))
                        }
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
    bound: Option<String>,
}

pub fn quoted<D>(d: D) -> String
where
    D: fmt::Debug,
{
    format!("{:?}", d)
}

pub struct _Enum {
    kw_pub: bool,
    name: String,
    variants: Vec<String>,
}

impl _Enum {
    pub fn kw_pub(&mut self) -> &mut Self {
        self.kw_pub = true;
        self
    }

    pub fn variant<D>(&mut self, d: D) -> &mut Self
    where
        D: Display,
    {
        self.variants.push(format!("{}", d));
        self
    }
}

pub fn _enum<S>(name: S) -> _Enum
where
    S: Into<String>,
{
    _Enum {
        name: name.into(),
        kw_pub: false,
        variants: Vec::new(),
    }
}

impl Display for _Enum {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        Scope::fmt(f, |s| {
            if self.kw_pub {
                s.write("pub ");
            }
            s.write("enum ").write(&self.name);
            if self.variants.is_empty() {
                s.write(" {}").lf();
            } else {
                s.in_block(|s| {
                    for variant in &self.variants {
                        s.write(variant).write(",").lf();
                    }
                });
            }
        })
    }
}

pub struct Derive {
    items: Vec<String>,
}

impl Derive {
    pub fn debug(mut self) -> Self {
        self.items.push("Debug".into());
        self
    }

    pub fn serialize(mut self) -> Self {
        self.items.push(Traits::Serialize());
        self
    }

    pub fn deserialize(mut self) -> Self {
        self.items.push(Traits::Deserialize());
        self
    }
}

impl Display for Derive {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "#[derive({items})]", items = self.items.join(", "))
    }
}

pub fn derive() -> Derive {
    Derive { items: Vec::new() }
}
