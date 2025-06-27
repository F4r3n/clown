use rustls::RootCertStore;
use rustls::pki_types::ServerName;
use std::net::ToSocketAddrs;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncRead};
use tokio::io::{AsyncWrite, AsyncWriteExt, BufReader, BufWriter};
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use tokio_rustls::TlsConnector;
use tokio_rustls::client::TlsStream;
pub struct ConnectionOption {
    address: String,
    port: u16,
    nickname: String,
    real_name: String,
    username: String,
    password: Option<String>,
}
pub trait AsyncStream: AsyncRead + AsyncWrite + Unpin + Send {}
impl<T: AsyncRead + AsyncWrite + Unpin + Send> AsyncStream for T {}
struct App {
    config: ConnectionOption,
    task: Option<tokio::task::JoinHandle<Result<(), anyhow::Error>>>,
}

impl App {
    pub fn new(config: ConnectionOption) -> Self {
        Self { config, task: None }
    }

    async fn connect(&mut self) -> Result<Box<dyn AsyncStream>, anyhow::Error> {
        rustls::crypto::ring::default_provider().install_default();
        let mut stream = self
            .establish_stream_tls(&self.config.address, self.config.port)
            .await?;
        //let (read_half, mut write_half) = split(stream);
        //let mut reader = BufReader::new(stream.r);

        stream.write_all(b"CAP LS 302\r\n").await?;

        if let Some(password) = &self.config.password {
            stream
                .write_all(format!("PASS {}\r\n", password).as_bytes())
                .await?;
        }

        stream
            .write_all(format!("NICK {}\r\n", self.config.nickname).as_bytes())
            .await?;

        stream
            .write_all(
                format!(
                    "USER {} 0 * :{}\r\n",
                    self.config.username, self.config.real_name
                )
                .as_bytes(),
            )
            .await?;

        stream.flush().await?;

        //capabilities_negotiation(&mut reader, &mut writer).await?;
        Ok(Box::new(stream))
    }

    async fn establish_stream(
        &self,
        in_address: &str,
        port: u16,
    ) -> Result<TcpStream, anyhow::Error> {
        let stream = TcpStream::connect(format!("{}:{}", in_address, port)).await?;
        Ok(stream)
    }

    async fn establish_stream_tls(
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

    async fn process<R, W>(
        mut reader: BufReader<R>,
        mut writer: BufWriter<W>,
        mut command_receiver: mpsc::Receiver<String>,
    ) -> Result<(), anyhow::Error>
    where
        R: AsyncRead + Unpin,
        W: AsyncWrite + Unpin,
    {
        let mut buffer = String::new();

        loop {
            tokio::select! {
                response = reader.read_line(&mut buffer) => {
                    match response {
                        Ok(0) => {
                            // Connection closed
                            //break;
                        }
                        Ok(_) => {
                            // Handle the response in buffer
                            println!("Received: {}", buffer.trim_end());
                            let line = buffer.trim_end();
                            println!("Received: {}", line);

                            // Handle PING immediately
                            if line.starts_with("PING") {
                                let response = line.replacen("PING", "PONG", 1);
                                writer.write_all(response.as_bytes()).await?;
                                writer.write_all(b"\r\n").await?;
                                writer.flush().await?;
                                println!("Sent: {}", response);
                            }

                            buffer.clear();
                        }
                        Err(e) => {
                            eprintln!("Read error: {}", e);
                            break;
                        }
                    }
                }
                command = command_receiver.recv() => {
                    if let Some(cmd) = command {
                        writer.write_all(cmd.as_bytes()).await?;
                        writer.flush().await?;
                    } else {
                        // All senders dropped, exit loop
                        //break;
                    }
                }
            }
        }
        Ok(())
    }

    pub async fn run(&mut self) -> Result<(), anyhow::Error> {
        let mut stream = self.connect().await?;
        let (command_sender, mut command_receiver) = mpsc::channel::<String>(32);

        let (reader_half, writer_half) = tokio::io::split(stream);
        let reader = BufReader::new(reader_half);
        let writer = BufWriter::new(writer_half);
        self.task = Some(tokio::spawn(Self::process(
            reader,
            writer,
            command_receiver,
        )));
        //self.process(reader, writer, command_receiver).await?;

        Ok(())
    }

    pub async fn stop(&mut self) {
        if let Some(task) = &self.task {
            task.abort();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_connect() -> anyhow::Result<()> {
        let option = ConnectionOption {
            address: "chat.freenode.net".into(),
            nickname: "farine".into(),
            password: None,
            port: 6697,
            real_name: "farine".into(),
            username: "farine".into(),
        };
        let mut app = App::new(option);
        app.connect().await?;
        Ok(())
    }
}
