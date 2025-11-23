use thiserror::Error;
use tokio::io;

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
    IO(#[from] io::Error),
    #[error("Cannot send command")]
    SendCommand,
    #[error("Cannot send Message")]
    SendMessage,
    #[error("unknown data store error")]
    Unknown,
}

#[derive(Error, Debug)]
pub enum ClownError {
    #[error(transparent)]
    IRCIOError(#[from] IRCIOError),
    #[error(transparent)]
    ConnectionError(#[from] ConnectionError),
}
