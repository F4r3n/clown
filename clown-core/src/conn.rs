use rustls::RootCertStore;
use rustls::pki_types::ServerName;
use std::net::ToSocketAddrs;
use std::sync::Arc;

use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;
use tokio_rustls::TlsConnector;
use tokio_rustls::client::TlsStream;

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
    ) -> Result<TlsStream<TcpStream>, anyhow::Error> {
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
        Ok(stream)
    }

    pub async fn connect_tls(&self) -> Result<TlsStream<TcpStream>, anyhow::Error> {
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
