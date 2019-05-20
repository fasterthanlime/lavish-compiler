use serde::{de::*, ser::*};
use std::{fmt, fmt::Debug};
use std::marker::{Send,PhantomData};

pub use erased_serde;
pub use serde_derive;

pub trait PendingRequests {
    fn get_pending(&self, id: u32) -> Option<&'static str>;
}

pub trait Atom: serde::Serialize + Debug + Sized + Send + 'static {
    fn method(&self) -> &'static str;
    fn deserialize(method: &str, de: &mut erased_serde::Deserializer)
        -> erased_serde::Result<Self>;
}

struct AtomApply<T: ?Sized>
where
    T: Atom,
{
    pub kind: String,
    pub phantom: PhantomData<T>,
}

impl<'de, T: ?Sized> DeserializeSeed<'de> for AtomApply<T>
where
    T: Atom,
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

struct AtomOptionApply<T: ?Sized>
where T: Atom,
{
    pub kind: String,
    pub phantom: PhantomData<T>,
}

impl<'de, T: ?Sized> DeserializeSeed<'de> for AtomOptionApply<T>
where T: Atom,
{
    type Value = Option<T>;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where D: Deserializer<'de>,
    {
        deserializer.deserialize_option(AtomOptionVisitor {
            kind: self.kind,
            phantom: PhantomData,
        })
    }
}

struct AtomOptionVisitor<T: ?Sized>
where T: Atom,
{
    pub kind: String,
    pub phantom: PhantomData<T>,
}

impl<'de, T: ?Sized> Visitor<'de> for AtomOptionVisitor<T>
where T: Atom,
{
    type Value = Option<T>;

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "a nullable msgpack-RPC payload (results, params, etc.)")
    }

    fn visit_none<E>(self) -> Result<Self::Value, E> {
        Ok(None)
    }

    fn visit_some<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where D: Deserializer<'de> {
        let mut erased = erased_serde::Deserializer::erase(deserializer);
        T::deserialize(&self.kind, &mut erased).map(Some).map_err(serde::de::Error::custom)
    }
}

#[derive(Debug)]
pub enum Message<P, NP, R>
where
    P: Atom,
    NP: Atom,
    R: Atom,
{
    Request {
        id: u32,
        params: P,
    },
    Response {
        id: u32,
        error: Option<String>,
        results: Option<R>,
    },
    Notification {
        params: NP,
    },
}

impl<P, NP, R> Message<P, NP, R>
where
    P: Atom,
    NP: Atom,
    R: Atom,
{
    pub fn request(id: u32, params: P) -> Self {
        Message::<P, NP, R>::Request { id, params }
    }

    pub fn notification(params: NP) -> Self {
        Message::<P, NP, R>::Notification { params }
    }

    pub fn response(id: u32, error: Option<String>, results: Option<R>) -> Self {
        Message::<P, NP, R>::Response { id, error, results }
    }
}

impl<P, NP, R> Serialize for Message<P, NP, R>
where
    P: Atom,
    NP: Atom,
    R: Atom,
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

impl<P, NP, R> Message<P, NP, R>
where
    P: Atom,
    NP: Atom,
    R: Atom,
{
    pub fn deserialize<'de, D>(d: D, pending: &'de dyn PendingRequests) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        d.deserialize_seq(MessageVisitor::<'de, P, NP, R> {
            pending,
            phantom: PhantomData,
        })
    }
}

struct MessageVisitor<'de, P, NP, R>
where
    P: Atom,
    NP: Atom,
    R: Atom,
{
    pending: &'de dyn PendingRequests,
    phantom: PhantomData<(P, NP, R)>,
}

impl<'de, P, NP, R> Visitor<'de> for MessageVisitor<'de, P, NP, R>
where
    P: Atom,
    NP: Atom,
    R: Atom,
{
    type Value = Message<P, NP, R>;

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "a valid msgpack-RPC message (ie. a sequence)")
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

                let seed = AtomApply::<P> {
                    kind: method,
                    phantom: std::marker::PhantomData,
                };
                let params = access
                    .next_element_seed(seed)?
                    .ok_or_else(|| missing("params"))?;

                Ok(Message::Request { id, params })
            }
            // Response
            1 => {
                let id = access.next_element::<u32>()?.ok_or_else(|| missing("id"))?;
                let error = access
                    .next_element::<Option<String>>()?
                    .ok_or_else(|| missing("error"))?;

                let method = self
                    .pending
                    .get_pending(id)
                    .ok_or_else(|| missing("no such pending request"))?;

                let seed = AtomOptionApply::<R> {
                    kind: method.into(),
                    phantom: std::marker::PhantomData,
                };
                let results = access
                    .next_element_seed(seed)?
                    .ok_or_else(|| missing("results"))?;

                Ok(Message::Response { id, error, results })
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

    impl Atom for Test {
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

        let pr = TestPendingRequests {};
        let m2: Message =
            Message::deserialize(&mut rmp_serde::Deserializer::from_slice(&buf1[..]), &pr).unwrap();
        println!("m2 = {:#?}", m2);

        let mut buf2: Vec<u8> = Vec::new();
        m2.serialize(&mut rmp_serde::Serializer::new_named(&mut buf2))
            .unwrap();

        assert_eq!(buf1, buf2);
    }

    struct TestPendingRequests {}

    impl PendingRequests for TestPendingRequests {
        fn get_pending(&self, _id: u32) -> Option<&'static str> {
            None
        }
    }
}