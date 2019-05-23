use super::super::{Atom, Message};
use super::Queue;

use serde::Serialize;
use std::io::Cursor;

use futures::lock::Mutex;
use std::sync::Arc;

use bytes::*;
use futures_codec::{Decoder, Encoder};

const MAX_MESSAGE_SIZE: usize = 128 * 1024;

pub struct Codec<P, NP, R>
where
    P: Atom,
    NP: Atom,
    R: Atom,
{
    buffer: Vec<u8>,
    queue: Arc<Mutex<Queue<P, NP, R>>>,
}

impl<P, NP, R> Codec<P, NP, R>
where
    P: Atom,
    NP: Atom,
    R: Atom,
{
    pub fn new(queue: Arc<Mutex<Queue<P, NP, R>>>) -> Self {
        let buffer: Vec<u8> = vec![0; MAX_MESSAGE_SIZE];
        Self { buffer, queue }
    }
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
        use std::io::{self, Write};

        // TODO: there's probably a way to do that without an additional buffer/copy
        let payload_slice = {
            let cursor = Cursor::new(&mut self.buffer[..]);
            let mut ser = rmp_serde::Serializer::new_named(cursor);
            item.serialize(&mut ser)
                .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
            let cursor = ser.into_inner();
            let written = cursor.position() as usize;
            &self.buffer[..written]
        };

        let mut length_buffer = vec![0; 16];
        let length_slice = {
            use serde::ser::Serializer;

            let cursor = Cursor::new(&mut length_buffer);
            let mut ser = rmp_serde::Serializer::new(cursor);
            ser.serialize_u64(payload_slice.len() as u64)
                .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

            let cursor = ser.into_inner();
            let written = cursor.position() as usize;
            &length_buffer[..written]
        };

        let total_len = length_slice.len() + payload_slice.len();
        dst.resize(total_len, 0);
        {
            println!(
                "[len={} bytes, payload={} bytes]",
                length_slice.len(),
                payload_slice.len()
            );
            let mut cursor = Cursor::new(&mut dst[..total_len]);
            cursor.write_all(length_slice)?;
            cursor.write_all(payload_slice)?;
        }

        Ok(())
    }
}

struct LengthVisitor {}

impl<'de> serde::de::Visitor<'de> for LengthVisitor {
    type Value = u64;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("a framed msgpack-rpc payload length")
    }

    fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(value)
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
        use serde::de::Deserializer;
        use std::io;

        if src.is_empty() {
            return Ok(None);
        }

        let (len_len, payload_len) = {
            let cursor = Cursor::new(&src[..]);
            let mut deser = rmp_serde::Deserializer::new(cursor);
            // FIXME: don't return error if we can't read the length,
            // just say we need more
            let payload_len = deser
                .deserialize_u64(LengthVisitor {})
                .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?
                as usize;
            let cursor = deser.into_inner();
            let len_len = cursor.position() as usize;
            (len_len, payload_len)
        };

        println!("len_len = {}", len_len);
        println!("payload_len = {}", payload_len);
        let total_len = len_len + payload_len;
        if src.len() < total_len {
            // need more data
            println!("has {}/{} needed", src.len(), total_len);
            return Ok(None);
        }

        {
            let cursor = Cursor::new(&src[len_len..total_len]);
            let mut deser = rmp_serde::Deserializer::from_read(cursor);
            if let Some(pr) = self.queue.try_lock() {
                let payload = Self::Item::deserialize(&mut deser, &*pr)
                    .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
                src.split_to(total_len);
                Ok(Some(payload))
            } else {
                // FIXME: futures_codec doesn't really fit our usecase :(
                panic!("could not acquire lock in decode");
            }
        }
    }
}
