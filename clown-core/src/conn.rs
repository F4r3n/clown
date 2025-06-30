use rustls::RootCertStore;
use rustls::pki_types::ServerName;
use std::net::ToSocketAddrs;
use std::pin::Pin;
use std::sync::Arc;

use tokio::io::{AsyncRead, AsyncWrite, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio_rustls::TlsConnector;

pub enum IRCStream {
    TLS(tokio_rustls::client::TlsStream<tokio::net::TcpStream>),
}

impl AsyncRead for IRCStream {
    fn poll_read(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut tokio::io::ReadBuf<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        match self.get_mut() {
            IRCStream::TLS(stream) => Pin::new(stream).poll_read(cx, buf),
        }
    }
}

impl AsyncWrite for IRCStream {
    fn is_write_vectored(&self) -> bool {
        match self {
            IRCStream::TLS(stream) => stream.get_ref().0.is_write_vectored(),
        }
    }

    fn poll_flush(
        self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), std::io::Error>> {
        match self.get_mut() {
            IRCStream::TLS(stream) => Pin::new(stream).poll_flush(cx),
        }
    }

    fn poll_shutdown(
        self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), std::io::Error>> {
        match self.get_mut() {
            IRCStream::TLS(stream) => Pin::new(stream).poll_shutdown(cx),
        }
    }

    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &[u8],
    ) -> std::task::Poll<Result<usize, std::io::Error>> {
        match self.get_mut() {
            IRCStream::TLS(stream) => Pin::new(stream).poll_write(cx, buf),
        }
    }

    fn poll_write_vectored(
        self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        bufs: &[std::io::IoSlice<'_>],
    ) -> std::task::Poll<Result<usize, std::io::Error>> {
        match self.get_mut() {
            IRCStream::TLS(stream) => Pin::new(stream).poll_write_vectored(cx, bufs),
        }
    }
}

#[derive(Debug)]
pub struct ConnectionConfig {
    pub address: String,
    pub port: u16,
    pub nickname: String,
    pub real_name: String,
    pub username: String,
    pub password: Option<String>,
}

#[derive(Debug)]
pub struct Connection {
    connection_config: ConnectionConfig,
}

impl Connection {
    pub fn new(config: ConnectionConfig) -> Self {
        Self {
            connection_config: config,
        }
    }

    async fn establish_stream(
        &self,
        in_address: &str,
        port: u16,
    ) -> Result<TcpStream, anyhow::Error> {
        let stream = TcpStream::connect(format!("{}:{}", in_address, port)).await?;
        Ok(stream)
    }

    pub async fn establish_stream_tls(
        &self,
        host: &str,
        port: u16,
    ) -> Result<IRCStream, anyhow::Error> {
        let addr = (host, port)
            .to_socket_addrs()?
            .next()
            .ok_or_else(|| std::io::Error::from(std::io::ErrorKind::NotFound))?;

        let mut root_cert_store = RootCertStore::empty();
        root_cert_store.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());

        let config = rustls::ClientConfig::builder()
            .with_root_certificates(root_cert_store)
            .with_no_client_auth(); // i guess this was previously the default?
        let connector = TlsConnector::from(Arc::new(config));
        let stream = TcpStream::connect(&addr).await?;

        let domain = ServerName::try_from(host)?.to_owned();
        let stream = connector.connect(domain, stream).await?;
        Ok(IRCStream::TLS(stream))
    }

    pub async fn connect_tls(&self) -> Result<IRCStream, anyhow::Error> {
        rustls::crypto::ring::default_provider().install_default();
        let mut stream = self
            .establish_stream_tls(&self.connection_config.address, self.connection_config.port)
            .await?;
        //let (read_half, mut write_half) = split(stream);
        //let mut reader = BufReader::new(stream.r);

        stream.write_all(b"CAP LS 302\r\n").await?;

        if let Some(password) = &self.connection_config.password {
            stream
                .write_all(format!("PASS {}\r\n", password).as_bytes())
                .await?;
        }

        stream
            .write_all(format!("NICK {}\r\n", self.connection_config.nickname).as_bytes())
            .await?;

        stream
            .write_all(
                format!(
                    "USER {} 0 * :{}\r\n",
                    self.connection_config.username, self.connection_config.real_name
                )
                .as_bytes(),
            )
            .await?;

        stream.flush().await?;

        //capabilities_negotiation(&mut reader, &mut writer).await?;
        Ok(stream)
    }
}
