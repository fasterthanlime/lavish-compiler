
use futures::task::SpawnError;

#[derive(Debug)]
pub enum Error {
    WrongResults,
    MissingResults,
    WrongMessageType,
    RemoteError(String),
    TransportError(String),
    SpawnError(SpawnError),
}

use std::fmt;
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:#?}", self)
    }
}

impl std::error::Error for Error {}

impl From<SpawnError> for Error {
    fn from(e: SpawnError) -> Self {
        Error::SpawnError(e)
    }
}