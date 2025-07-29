use crate::error::ConnectionError;
use rustls::RootCertStore;
use rustls::pki_types::ServerName;
use std::net::ToSocketAddrs;
use std::pin::Pin;
use std::sync::Arc;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::net::TcpStream;
use tokio_rustls::TlsConnector;
pub enum IRCStream {
    TLS(Box<tokio_rustls::client::TlsStream<tokio::net::TcpStream>>),
    PLAIN(tokio::net::TcpStream),
    #[cfg(test)]
    MOCK(test::StreamMock),
}

impl AsyncRead for IRCStream {
    fn poll_read(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut tokio::io::ReadBuf<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        match self.get_mut() {
            IRCStream::TLS(stream) => Pin::new(stream).poll_read(cx, buf),
            IRCStream::PLAIN(stream) => Pin::new(stream).poll_read(cx, buf),

            #[cfg(test)]
            IRCStream::MOCK(mock) => Pin::new(mock).poll_read(cx, buf),
        }
    }
}

impl AsyncWrite for IRCStream {
    fn is_write_vectored(&self) -> bool {
        match self {
            IRCStream::TLS(stream) => stream.get_ref().0.is_write_vectored(),
            IRCStream::PLAIN(stream) => stream.is_write_vectored(),

            #[cfg(test)]
            IRCStream::MOCK(mock) => mock.is_write_vectored(),
        }
    }

    fn poll_flush(
        self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), std::io::Error>> {
        match self.get_mut() {
            IRCStream::TLS(stream) => Pin::new(stream).poll_flush(cx),
            IRCStream::PLAIN(stream) => Pin::new(stream).poll_flush(cx),

            #[cfg(test)]
            IRCStream::MOCK(mock) => Pin::new(mock).poll_flush(cx),
        }
    }

    fn poll_shutdown(
        self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), std::io::Error>> {
        match self.get_mut() {
            IRCStream::TLS(stream) => Pin::new(stream).poll_shutdown(cx),
            IRCStream::PLAIN(stream) => Pin::new(stream).poll_shutdown(cx),

            #[cfg(test)]
            IRCStream::MOCK(mock) => Pin::new(mock).poll_shutdown(cx),
        }
    }

    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &[u8],
    ) -> std::task::Poll<Result<usize, std::io::Error>> {
        match self.get_mut() {
            IRCStream::TLS(stream) => Pin::new(stream).poll_write(cx, buf),
            IRCStream::PLAIN(stream) => Pin::new(stream).poll_write(cx, buf),

            #[cfg(test)]
            IRCStream::MOCK(mock) => Pin::new(mock).poll_write(cx, buf),
        }
    }

    fn poll_write_vectored(
        self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        bufs: &[std::io::IoSlice<'_>],
    ) -> std::task::Poll<Result<usize, std::io::Error>> {
        match self.get_mut() {
            IRCStream::TLS(stream) => Pin::new(stream).poll_write_vectored(cx, bufs),
            IRCStream::PLAIN(stream) => Pin::new(stream).poll_write_vectored(cx, bufs),

            #[cfg(test)]
            IRCStream::MOCK(mock) => Pin::new(mock).poll_write_vectored(cx, bufs),
        }
    }
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone)]
pub struct ConnectionConfig {
    pub address: String,
    pub port: u16,
}

#[derive(Debug)]
pub struct Connection {
    connection_config: ConnectionConfig,
}

impl Connection {
    pub fn new(config: &ConnectionConfig) -> Self {
        Self {
            connection_config: config.clone(),
        }
    }

    async fn establish_stream(
        &self,
        in_address: &str,
        port: u16,
    ) -> Result<IRCStream, ConnectionError> {
        let stream = TcpStream::connect(format!("{in_address}:{port}")).await?;
        Ok(IRCStream::PLAIN(stream))
    }

    async fn establish_stream_tls(
        &self,
        host: &str,
        port: u16,
    ) -> Result<IRCStream, ConnectionError> {
        let addr = (host, port)
            .to_socket_addrs()?
            .next()
            .ok_or_else(|| std::io::Error::from(std::io::ErrorKind::NotFound))?;

        let mut root_cert_store = RootCertStore::empty();
        root_cert_store.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());

        let config = rustls::ClientConfig::builder()
            .with_root_certificates(root_cert_store)
            .with_no_client_auth();
        let connector = TlsConnector::from(Arc::new(config));
        let stream = TcpStream::connect(&addr).await?;

        let domain = ServerName::try_from(host)
            .map_err(|_err| ConnectionError::InvalidDNS)?
            .to_owned();
        let stream = connector.connect(domain, stream).await?;
        Ok(IRCStream::TLS(Box::new(stream)))
    }

    pub async fn connect(&self) -> Result<IRCStream, ConnectionError> {
        let _result = rustls::crypto::ring::default_provider().install_default();
        let stream = if self.connection_config.port == 6697 {
            self.establish_stream_tls(&self.connection_config.address, self.connection_config.port)
                .await?
        } else {
            self.establish_stream(&self.connection_config.address, self.connection_config.port)
                .await?
        };
        Ok(stream)
    }
}

#[cfg(test)]
pub mod test {
    use std::collections::VecDeque;
    use std::pin::Pin;
    use std::task::{Context, Poll};
    use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};

    pub enum Action {
        Item(Vec<u8>),
        Wait(u8),
    }

    pub struct StreamMock {
        items: VecDeque<Action>,
        current: Option<Vec<u8>>,
        pos: usize,
    }

    impl StreamMock {
        pub fn new(actions: Vec<Action>) -> Self {
            Self {
                items: actions.into(),
                current: None,
                pos: 0,
            }
        }
        fn next_action(&mut self) -> Option<Action> {
            self.items.pop_front()
        }
    }

    impl AsyncRead for StreamMock {
        fn poll_read(
            mut self: Pin<&mut Self>,
            _cx: &mut Context<'_>,
            buf: &mut ReadBuf<'_>,
        ) -> std::task::Poll<std::io::Result<()>> {
            loop {
                if let Some(current) = self.current.take() {
                    let remaining = &current[self.pos..];
                    let to_read = std::cmp::min(remaining.len(), buf.remaining());
                    buf.put_slice(&remaining[..to_read]);
                    self.pos += to_read;
                    if self.pos < current.len() {
                        self.current = Some(current);
                    } else {
                        self.pos = 0;
                    }
                    return Poll::Ready(Ok(()));
                } else {
                    match self.next_action() {
                        Some(Action::Item(data)) => {
                            self.current = Some(data);
                            self.pos = 0;
                        }
                        Some(Action::Wait(_)) => {
                            return Poll::Pending;
                        }
                        None => return Poll::Ready(Ok(())),
                    }
                }
            }
        }
    }

    impl AsyncWrite for StreamMock {
        fn poll_write(
            mut self: Pin<&mut Self>,
            _cx: &mut Context<'_>,
            buf: &[u8],
        ) -> Poll<std::io::Result<usize>> {
            self.items.push_back(Action::Item(buf.to_vec()));
            Poll::Ready(Ok(buf.len()))
        }

        fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
            Poll::Ready(Ok(()))
        }

        fn poll_shutdown(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
            Poll::Ready(Ok(()))
        }

        fn is_write_vectored(&self) -> bool {
            false
        }

        fn poll_write_vectored(
            self: Pin<&mut Self>,
            _cx: &mut Context<'_>,
            bufs: &[std::io::IoSlice<'_>],
        ) -> Poll<std::io::Result<usize>> {
            let total_len: usize = bufs.iter().map(|b| b.len()).sum();
            Poll::Ready(Ok(total_len))
        }
    }
}

#[cfg(test)]
mod tests {

    use tokio::io::BufReader;

    use crate::conn::test::Action;
    use crate::conn::test::StreamMock;
    use tokio::io::AsyncBufReadExt;
    #[tokio::test]
    async fn test_mock_simple() -> anyhow::Result<()> {
        let stream_mock = StreamMock::new(vec![
            Action::Item("test\n".as_bytes().to_vec()),
            Action::Item("HELLO\n".as_bytes().to_vec()),
        ]);
        let mut reader = BufReader::new(stream_mock);
        let mut line = String::new();

        reader.read_line(&mut line).await.unwrap();
        assert_eq!(line, "test\n".to_string());

        line.clear();
        reader.read_line(&mut line).await.unwrap();
        assert_eq!(line, "HELLO\n".to_string());

        Ok(())
    }
}
