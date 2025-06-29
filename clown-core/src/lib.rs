use tokio::io::BufReader;
use tokio::io::{AsyncBufReadExt, AsyncRead};
use tokio::io::{AsyncWrite, AsyncWriteExt, BufWriter};
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use tokio_rustls::client::TlsStream;
pub mod conn;

struct CommandReceiver {
    pub inner: mpsc::UnboundedReceiver<String>,
}

#[derive(Clone)]
struct Sender {
    inner: Option<mpsc::UnboundedSender<String>>,
}

impl Sender {
    pub fn new() -> Self {
        Self { inner: None }
    }
}

impl Sender {
    /// The core processing loop. No 'static bound required on handler!
    async fn process<R, W, F>(
        &self,
        mut reader: BufReader<R>,
        mut writer: BufWriter<W>,
        mut command_receiver: CommandReceiver,
        message_handler: &mut F,
    ) -> anyhow::Result<()>
    where
        R: AsyncRead + Unpin,
        W: AsyncWrite + Unpin,
        F: FnMut(String),
    {
        let mut buffer = String::new();

        loop {
            tokio::select! {
                response = reader.read_line(&mut buffer) => {
                    match response {
                        Ok(0) => break, // Connection closed
                        Ok(_) => {
                            let line = buffer.trim_end().to_string();
                            buffer.clear();

                            // Handle PING immediately
                            if line.starts_with("PING") {
                                let response = line.replacen("PING", "PONG", 1);
                                writer.write_all(response.as_bytes()).await?;
                                writer.write_all(b"\r\n").await?;
                                writer.flush().await?;
                            }

                            // Call user's handler (can borrow local data!)
                            message_handler(line);
                        }
                        Err(e) => {
                            eprintln!("Read error: {}", e);
                            break;
                        }
                    }
                }
                command = command_receiver.inner.recv() => {
                    if let Some(cmd) = command {
                        writer.write_all(cmd.as_bytes()).await?;
                        writer.flush().await?;
                    } else {
                        break; // Command channel closed
                    }
                }
            }
        }
        Ok(())
    }

    pub fn create_command_receiver(&mut self) -> anyhow::Result<CommandReceiver> {
        let (command_sender, command_receiver) = mpsc::unbounded_channel::<String>();
        self.inner = Some(command_sender);
        Ok(CommandReceiver {
            inner: command_receiver,
        })
    }

    pub async fn run<F>(
        &self,
        tls_stream: TlsStream<TcpStream>,
        mut message_handler: F,
        command_receiver: CommandReceiver,
    ) -> anyhow::Result<()>
    where
        F: FnMut(String),
    {
        let (reader_half, writer_half) = tokio::io::split(tls_stream);

        let reader = BufReader::new(reader_half);
        let writer = BufWriter::new(writer_half);

        self.process(reader, writer, command_receiver, &mut message_handler)
            .await
    }
}
use conn::Connection;
use conn::ConnectionConfig;

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_connect() -> anyhow::Result<()> {
        let option = ConnectionConfig {
            address: "chat.freenode.net".into(),
            nickname: "farine".into(),
            password: None,
            port: 6697,
            real_name: "farine".into(),
            username: "farine".into(),
        };
        let conn = Connection::new(option);
        let mut sender = Sender::new();
        let receiver = sender.create_command_receiver()?;
        let tls = conn.connect_tls().await?;

        let sender_clone = sender.clone();

        // Spawn processing in background
        tokio::spawn(async move {
            sender_clone
                .run(
                    tls,
                    |msg| {
                        println!("Received: {}", msg);
                    },
                    receiver,
                )
                .await
                .expect("Client failed");
        });

        // Send commands after delay
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
        if let Some(tx) = &sender.inner {
            tx.send("NICK mybot\r\n".to_string())?;
            tx.send("USER mybot 0 * :Rust Bot\r\n".to_string())?;
        }
        Ok(())
    }
}
