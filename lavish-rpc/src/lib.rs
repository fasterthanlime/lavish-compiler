use serde::{de::*, ser::*};
use std::{fmt, fmt::Debug};

//----------------

pub trait Proto: serde::Serialize + Debug + Sized {
    fn method(&self) -> &'static str;
    fn deserialize(method: &str, de: &mut erased_serde::Deserializer)
        -> erased_serde::Result<Self>;
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
    type Value = T;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        let mut erased = erased_serde::Deserializer::erase(deserializer);
        T::deserialize(&self.kind, &mut erased).map_err(serde::de::Error::custom)
    }
}

#[derive(Debug)]
pub enum Message<P, NP, R>
where
    P: Proto,
    NP: Proto,
    R: Proto,
{
    Request {
        id: u32,
        params: P,
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

impl<P, NP, R> Message<P, NP, R>
where
    P: Proto,
    NP: Proto,
    R: Proto,
{
    pub fn request(id: u32, params: P) -> Self {
        Message::<P, NP, R>::Request { id, params }
    }
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
                seq.serialize_element(params)?;
                seq.end()
            }
            Message::Response {
                id, error, results, ..
            } => {
                let mut seq = s.serialize_seq(Some(4))?;
                seq.serialize_element(&1)?;
                seq.serialize_element(&id)?;
                seq.serialize_element(&error)?;
                seq.serialize_element(results)?;
                seq.end()
            }
            Message::Notification { params, .. } => {
                let mut seq = s.serialize_seq(Some(3))?;
                seq.serialize_element(&2)?;
                seq.serialize_element(params.method())?;
                seq.serialize_element(params)?;
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
                println!("id = {}", id);
                let method = access
                    .next_element::<String>()?
                    .ok_or_else(|| missing("method"))?;
                println!("method = {}", method);

                let seed = ProtoApply::<P> {
                    kind: method,
                    phantom: std::marker::PhantomData,
                };
                let params = access
                    .next_element_seed(seed)?
                    .ok_or_else(|| missing("params"))?;
                println!("params = {:#?}", params);

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
    #[serde(untagged)]
    enum Test {
        Foo(TestFoo),
        Bar(TestBar),
    }

    #[derive(Serialize, Deserialize, Debug)]
    struct TestFoo {
        val: i64,
    }

    #[derive(Serialize, Deserialize, Debug)]
    struct TestBar {
        val: String,
        #[serde(with = "serde_bytes")]
        bs: Vec<u8>,
    }

    type Message = super::Message<Test, Test, Test>;

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
        ) -> erased_serde::Result<Self> {
            match method {
                "Foo" => Ok(Test::Foo(erased_serde::deserialize::<TestFoo>(de)?)),
                "Bar" => Ok(Test::Bar(erased_serde::deserialize::<TestBar>(de)?)),
                _ => Err(erased_serde::Error::custom(format!(
                    "unknown method: {}",
                    method
                ))),
            }
        }
    }

    #[test]
    fn internal() {
        cycle(Message::request(420, Test::Foo(TestFoo { val: 69 })));
        cycle(Message::request(
            420,
            Test::Bar(TestBar {
                val: "success!".into(),
                bs: vec![0x0, 0x15, 0x93],
            }),
        ));
    }

    fn cycle(m1: Message) {
        println!("m1 = {:#?}", m1);

        let mut buf1: Vec<u8> = Vec::new();
        m1.serialize(&mut rmp_serde::Serializer::new_named(&mut buf1))
            .unwrap();

        let m2: Message = rmp_serde::decode::from_slice(&buf1[..]).unwrap();
        println!("m2 = {:#?}", m2);

        let mut buf2: Vec<u8> = Vec::new();
        m2.serialize(&mut rmp_serde::Serializer::new_named(&mut buf2))
            .unwrap();

        assert_eq!(buf1, buf2);
    }
}
