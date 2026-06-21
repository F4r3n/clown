use thiserror::Error;
use tokio::io;
use tokio_util::codec::LinesCodecError;

#[derive(Error, Debug)]
pub enum ConnectionError {
    #[error("Cannot establish TCP")]
    ConnectTCP(#[from] io::Error),
    #[error("DNS cannot be created")]
    InvalidDNS,
    #[error("unknown data store error")]
    Unknown,
}

#[derive(Error, Debug)]
pub enum IRCIOError {
    #[error("IO error")]
    CodecError(#[from] LinesCodecError),
    #[error("Cannot send command")]
    IO(#[from] io::Error),
    #[error("Cannot send command")]
    SendCommand,
    #[error("Cannot send Message")]
    SendMessage,
    #[error("Timeout")]
    Timeout,
    #[error("unknown data store error")]
    Unknown,
    #[error("Uninitialized")]
    Uninitialized,
}

#[derive(Error, Debug)]
pub enum ClownError {
    #[error(transparent)]
    IRCIOError(#[from] IRCIOError),
    #[error(transparent)]
    ConnectionError(#[from] ConnectionError),
}
