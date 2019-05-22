use super::{Atom, Error, Message, PendingRequests};

use serde::Serialize;
use std::io::Cursor;
use std::marker::{PhantomData, Unpin};

use futures::lock::Mutex;
use std::collections::HashMap;
use std::sync::Arc;

use bytes::*;
use futures::channel::{mpsc, oneshot};
use futures::executor;
use futures::prelude::*;
use futures_codec::{Decoder, Encoder, Framed};

use futures::task::SpawnExt;

pub trait IO: AsyncRead + AsyncWrite + Send + Sized + Unpin + 'static {}
impl<T> IO for T where T: AsyncRead + AsyncWrite + Send + Sized + Unpin + 'static {}

#[derive(Clone, Copy)]
pub struct Protocol<P, NP, R>
where
    P: Atom,
    NP: Atom,
    R: Atom,
{
    phantom: PhantomData<(P, NP, R)>,
}

impl<P, NP, R> Protocol<P, NP, R>
where
    P: Atom,
    NP: Atom,
    R: Atom,
{
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self {
            phantom: PhantomData,
        }
    }
}

pub trait Handler<P, NP, R, FT>: Sync + Send
where
    P: Atom,
    NP: Atom,
    R: Atom,
    FT: Future<Output = Result<R, Error>> + Send + 'static,
{
    fn handle(&self, h: Handle<P, NP, R>, params: P) -> FT;
}

impl<P, NP, R, F, FT> Handler<P, NP, R, FT> for F
where
    P: Atom,
    R: Atom,
    NP: Atom,
    F: (Fn(Handle<P, NP, R>, P) -> FT) + Send + Sync,
    FT: Future<Output = Result<R, Error>> + Send + 'static,
{
    fn handle(&self, h: Handle<P, NP, R>, params: P) -> FT {
        self(h, params)
    }
}

pub struct Handle<P, NP, R>
where
    P: Atom,
    NP: Atom,
    R: Atom,
{
    queue: Arc<Mutex<Queue<P, NP, R>>>,
    sink: mpsc::Sender<Message<P, NP, R>>,
}

impl<P, NP, R> Handle<P, NP, R>
where
    P: Atom,
    NP: Atom,
    R: Atom,
{
    fn clone(&self) -> Self {
        Handle {
            queue: self.queue.clone(),
            sink: self.sink.clone(),
        }
    }

    #[allow(clippy::needless_lifetimes)]
    pub async fn call_raw(
        &self,
        params: P,
    ) -> Result<Message<P, NP, R>, Box<dyn std::error::Error>> {
        let id = {
            let mut queue = self.queue.lock().await;
            queue.next_id()
        };

        let method = params.method();
        let m = Message::Request { id, params };

        let (tx, rx) = oneshot::channel::<Message<P, NP, R>>();
        let in_flight = InFlightRequest { method, tx };
        {
            let mut queue = self.queue.lock().await;
            queue.in_flight_requests.insert(id, in_flight);
        }

        {
            let mut sink = self.sink.clone();
            sink.send(m).await?;
        }
        Ok(rx.await?)
    }

    #[allow(clippy::needless_lifetimes)]
    pub async fn call<D, RR>(&self, params: P, downgrade: D) -> Result<RR, Error>
    where
        D: Fn(R) -> Option<RR>,
    {
        match self.call_raw(params).await {
            Ok(m) => match m {
                Message::Response { results, error, .. } => {
                    if let Some(error) = error {
                        Err(Error::RemoteError(error))
                    } else if let Some(results) = results {
                        downgrade(results).ok_or_else(|| Error::WrongResults)
                    } else {
                        Err(Error::MissingResults)
                    }
                }
                _ => Err(Error::WrongMessageType),
            },
            Err(msg) => Err(Error::TransportError(format!("{:#?}", msg))),
        }
    }
}

pub struct System<P, NP, R>
where
    P: Atom,
    NP: Atom,
    R: Atom,
{
    handle: Handle<P, NP, R>,
}

impl<P, NP, R> System<P, NP, R>
where
    P: Atom,
    NP: Atom,
    R: Atom,
{
    pub fn new<T, H, FT>(
        protocol: Protocol<P, NP, R>,
        handler: Option<H>,
        io: T,
        mut pool: executor::ThreadPool,
    ) -> Result<Self, Error>
    where
        T: IO,
        H: Handler<P, NP, R, FT> + 'static,
        FT: Future<Output = Result<R, Error>> + Send + 'static,
    {
        let queue = Arc::new(Mutex::new(Queue::new(protocol)));

        let codec = Codec {
            queue: queue.clone(),
        };
        let framed = Framed::new(io, codec);
        let (mut sink, mut stream) = framed.split();
        let (tx, mut rx) = mpsc::channel(128);

        let handle = Handle::<P, NP, R> {
            queue: queue.clone(),
            sink: tx,
        };

        let system = System {
            handle: handle.clone(),
        };

        pool.clone().spawn(async move {
            while let Some(m) = rx.next().await {
                sink.send(m).await.unwrap();
            }
        })?;

        pool.clone()
            .spawn(async move {
                let handler = Arc::new(handler);

                while let Some(m) = stream.next().await {
                    let res =
                        m.map(|m| pool.spawn(handle_message(m, handler.clone(), handle.clone())));
                    if let Err(e) = res {
                        eprintln!("message stream error: {:#?}", e);
                    }
                }
            })
            .map_err(Error::SpawnError)?;

        Ok(system)
    }

    pub fn handle(&self) -> Handle<P, NP, R> {
        self.handle.clone()
    }
}

async fn handle_message<P, NP, R, H, FT>(
    inbound: Message<P, NP, R>,
    handler: Arc<Option<H>>,
    mut handle: Handle<P, NP, R>,
) where
    P: Atom,
    NP: Atom,
    R: Atom,
    H: Handler<P, NP, R, FT>,
    FT: Future<Output = Result<R, Error>> + Send + 'static,
{
    match inbound {
        Message::Request { id, params } => {
            let m = match handler.as_ref() {
                Some(handler) => match handler.handle(handle.clone(), params).await {
                    Ok(results) => Message::Response::<P, NP, R> {
                        id,
                        results: Some(results),
                        error: None,
                    },
                    Err(error) => Message::Response::<P, NP, R> {
                        id,
                        results: None,
                        error: Some(format!("internal error: {:#?}", error)),
                    },
                },
                _ => Message::Response {
                    id,
                    results: None,
                    error: Some("no method handler".into()),
                },
            };
            handle.sink.send(m).await.unwrap();
        }
        Message::Response { id, error, results } => {
            if let Some(in_flight) = {
                let mut queue = handle.queue.lock().await;
                queue.in_flight_requests.remove(&id)
            } {
                in_flight
                    .tx
                    .send(Message::Response { id, error, results })
                    .unwrap();
            }
        }
        Message::Notification { .. } => unimplemented!(),
    };
}

pub struct Codec<P, NP, R>
where
    P: Atom,
    NP: Atom,
    R: Atom,
{
    queue: Arc<Mutex<Queue<P, NP, R>>>,
}

impl<P, NP, R> Encoder for Codec<P, NP, R>
where
    P: Atom,
    NP: Atom,
    R: Atom,
{
    type Item = Message<P, NP, R>;
    type Error = std::io::Error;

    fn encode(&mut self, item: Self::Item, dst: &mut BytesMut) -> Result<(), Self::Error> {
        // TODO: check/improve resize logic
        let mut len = std::cmp::max(128, dst.capacity());
        dst.resize(len, 0);

        loop {
            let (cursor, res) = {
                let cursor = Cursor::new(&mut dst[..len]);
                let mut ser = rmp_serde::Serializer::new_named(cursor);
                let res = item.serialize(&mut ser);
                (ser.into_inner(), res)
            };
            use rmp_serde::encode::Error as EncErr;

            match res {
                Ok(_) => {
                    let pos = cursor.position();
                    dst.resize(pos as usize, 0);
                    return Ok(());
                }
                Err(EncErr::InvalidValueWrite(_)) => {
                    len *= 2;
                    dst.resize(len, 0);
                    continue;
                }
                Err(e) => return Err(std::io::Error::new(std::io::ErrorKind::Other, e)),
            }
        }
    }
}

impl<P, NP, R> Decoder for Codec<P, NP, R>
where
    P: Atom,
    NP: Atom,
    R: Atom,
{
    type Item = Message<P, NP, R>;
    type Error = std::io::Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        if src.is_empty() {
            return Ok(None);
        }

        let (pos, res) = {
            let cursor = Cursor::new(&src[..]);
            let mut deser = rmp_serde::Deserializer::from_read(cursor);
            let res = {
                if let Some(pr) = self.queue.try_lock() {
                    Self::Item::deserialize(&mut deser, &*pr)
                } else {
                    // FIXME: futures_codec doesn't fit the bill
                    panic!("could not acquire lock in decode");
                }
            };
            (deser.position(), res)
        };

        use rmp_serde::decode::Error as DecErr;
        let need_more = || Ok(None);

        match res {
            Ok(m) => {
                src.split_to(pos as usize);
                Ok(Some(m))
            }
            Err(DecErr::InvalidDataRead(_)) => need_more(),
            Err(DecErr::InvalidMarkerRead(_)) => need_more(),
            Err(DecErr::Syntax(_)) => need_more(),
            Err(e) => Err(std::io::Error::new(std::io::ErrorKind::Other, e)),
        }
    }
}

struct InFlightRequest<P, NP, R>
where
    P: Atom,
    NP: Atom,
    R: Atom,
{
    method: &'static str,
    tx: oneshot::Sender<Message<P, NP, R>>,
}

struct Queue<P, NP, R>
where
    P: Atom,
    NP: Atom,
    R: Atom,
{
    id: u32,
    in_flight_requests: HashMap<u32, InFlightRequest<P, NP, R>>,
}

impl<P, NP, R> Queue<P, NP, R>
where
    P: Atom,
    NP: Atom,
    R: Atom,
{
    fn new(_protocol: Protocol<P, NP, R>) -> Self {
        Queue {
            id: 0,
            in_flight_requests: HashMap::new(),
        }
    }

    fn next_id(&mut self) -> u32 {
        let res = self.id;
        self.id += 1;
        res
    }
}

impl<P, NP, R> PendingRequests for Queue<P, NP, R>
where
    P: Atom,
    NP: Atom,
    R: Atom,
{
    fn get_pending(&self, id: u32) -> Option<&'static str> {
        self.in_flight_requests.get(&id).map(|req| req.method)
    }
}
