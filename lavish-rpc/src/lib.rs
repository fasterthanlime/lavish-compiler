use serde::{de::*, ser::*};
use std::{fmt, fmt::Debug};

//----------------

pub trait Proto: serde::Serialize + Debug + Sized {
    fn method(&self) -> &'static str;
    fn deserialize(
        method: &str,
        de: &mut erased_serde::Deserializer,
    ) -> erased_serde::Result<Box<Self>>;
}

struct ProtoApply<T: ?Sized>
where
    T: Proto,
{
    pub kind: String,
    pub phantom: std::marker::PhantomData<T>,
}

impl<'de, T: ?Sized> DeserializeSeed<'de> for ProtoApply<T>
where
    T: Proto,
{
    type Value = Box<T>;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        let mut erased = erased_serde::Deserializer::erase(deserializer);
        T::deserialize(&self.kind, &mut erased).map_err(serde::de::Error::custom)
    }
}

pub enum Message<P, NP, R>
where
    P: Proto,
    NP: Proto,
    R: Proto,
{
    Request {
        id: u32,
        params: Box<P>,
    },
    Response {
        id: u32,
        error: Option<String>,
        results: R,
    },
    Notification {
        params: NP,
    },
}

impl<P, NP, R> Serialize for Message<P, NP, R>
where
    P: Proto,
    NP: Proto,
    R: Proto,
{
    fn serialize<S>(&self, s: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Message::Request { id, params, .. } => {
                let mut seq = s.serialize_seq(Some(4))?;
                seq.serialize_element(&0)?;
                seq.serialize_element(&id)?;
                seq.serialize_element(params.method())?;
                seq.serialize_element(&params)?;
                seq.end()
            }
            Message::Response {
                id, error, results, ..
            } => {
                let mut seq = s.serialize_seq(Some(4))?;
                seq.serialize_element(&1)?;
                seq.serialize_element(&id)?;
                seq.serialize_element(&error)?;
                seq.serialize_element(&results)?;
                seq.end()
            }
            Message::Notification { params, .. } => {
                let mut seq = s.serialize_seq(Some(3))?;
                seq.serialize_element(&2)?;
                seq.serialize_element(params.method())?;
                seq.serialize_element(&params)?;
                seq.end()
            }
        }
    }
}

impl<'de, P, NP, R> Deserialize<'de> for Message<P, NP, R>
where
    P: Proto,
    NP: Proto,
    R: Proto,
{
    fn deserialize<D>(d: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        d.deserialize_seq(MessageVisitor::<P, NP, R> {
            _p: std::marker::PhantomData,
            _np: std::marker::PhantomData,
            _r: std::marker::PhantomData,
        })
    }
}

struct MessageVisitor<P, NP, R>
where
    P: Proto,
    NP: Proto,
    R: Proto,
{
    _p: std::marker::PhantomData<P>,
    _np: std::marker::PhantomData<NP>,
    _r: std::marker::PhantomData<R>,
}

impl<'de, P, NP, R> Visitor<'de> for MessageVisitor<P, NP, R>
where
    P: Proto,
    NP: Proto,
    R: Proto,
{
    type Value = Message<P, NP, R>;

    fn expecting(&self, _formatter: &mut fmt::Formatter) -> fmt::Result {
        unimplemented!()
    }

    fn visit_seq<S>(self, mut access: S) -> Result<Self::Value, S::Error>
    where
        S: SeqAccess<'de>,
    {
        use serde::de::Error;
        let missing = |field: &str| -> S::Error {
            S::Error::custom(format!("invalid msgpack-RPC message: missing {}", field))
        };

        let typ = access
            .next_element::<u32>()?
            .ok_or_else(|| missing("type"))?;

        match typ {
            // Request
            0 => {
                let id = access.next_element::<u32>()?.ok_or_else(|| missing("id"))?;
                let method = access
                    .next_element::<String>()?
                    .ok_or_else(|| missing("method"))?;

                let seed = ProtoApply {
                    kind: method,
                    phantom: std::marker::PhantomData,
                };
                let params = access
                    .next_element_seed(seed)?
                    .ok_or_else(|| missing("params"))?;

                Ok(Message::Request { id, params })
            }
            _ => unimplemented!(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_derive::*;

    #[derive(Serialize, Debug)]
    enum Test {
        Foo(TestFoo),
        Bar(TestBar),
    }

    #[derive(Serialize, Deserialize, Debug)]
    struct TestFoo {
        val: String,
    }

    #[derive(Serialize, Deserialize, Debug)]
    struct TestBar {
        val: i64,
    }

    impl Proto for Test {
        fn method(&self) -> &'static str {
            match self {
                Test::Foo(_) => "Foo",
                Test::Bar(_) => "Bar",
            }
        }

        fn deserialize(
            method: &str,
            de: &mut erased_serde::Deserializer,
        ) -> erased_serde::Result<Box<Self>> {
            match method {
                "Foo" => Ok(Box::new(Test::Foo(erased_serde::deserialize::<TestFoo>(
                    de,
                )?))),
                "Bar" => Ok(Box::new(Test::Bar(erased_serde::deserialize::<TestBar>(
                    de,
                )?))),
                _ => Err(erased_serde::Error::custom(format!(
                    "unknown method: {}",
                    method
                ))),
            }
        }
    }

    #[test]
    fn internal() {
        let m = Message::<Test, Test, Test>::Request {
            id: 0,
            params: Box::new(Test::Bar(TestBar { val: 420 })),
        };
    }
}
