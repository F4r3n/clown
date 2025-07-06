use crate::client::State;
use crate::command::CommandReceiver;
use crate::command::{self, Command};
use crate::message::{MessageReceiver, MessageSender};
use clown_parser::{Message, create_message};
use std::sync::Arc;
use tokio::io::BufReader;
use tokio::io::{AsyncBufReadExt, AsyncRead};
use tokio::io::{AsyncWrite, AsyncWriteExt, BufWriter};
use tokio::sync::mpsc;
pub struct Outgoing {
    receiver: Option<CommandReceiver>,
    message_sender: Option<MessageSender>,
}

impl Outgoing {
    pub fn new() -> Self {
        Self {
            receiver: None,
            message_sender: None,
        }
    }

    pub async fn process<R, W>(
        &mut self,
        mut reader: BufReader<R>,
        mut writer: BufWriter<W>,
        state: Arc<State>,
    ) -> anyhow::Result<()>
    where
        R: AsyncRead + Unpin,
        W: AsyncWrite + Unpin,
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
                            if let Some(sender) = &self.message_sender {
                                if let Ok(message) = create_message(line.as_bytes())
                                {
                            sender.inner.send(message)?;

                                }
                            }
                        }
                        Err(e) => {
                            eprintln!("Read error: {}", e);
                            break;
                        }
                    }
                }
                command = self.receiver.as_mut().unwrap().inner.recv() => {
                    if let Some(cmd) = command {
                        writer.write_all(cmd.as_bytes().as_slice()).await?;
                        writer.flush().await?;
                    } else {
                        break; // Command channel closed
                    }
                }
            }
        }
        Ok(())
    }

    pub fn create_outgoing(&mut self) -> (Sender, MessageReceiver) {
        let (command_sender, command_receiver) = mpsc::unbounded_channel::<command::Command>();
        let (message_sender, message_receiver) = mpsc::unbounded_channel::<Message>();
        self.receiver = Some(CommandReceiver {
            inner: command_receiver,
        });
        self.message_sender = Some(MessageSender {
            inner: message_sender,
        });
        (
            Sender {
                inner: Some(command_sender),
            },
            MessageReceiver {
                inner: message_receiver,
            },
        )
    }
}

#[derive(Clone)]
pub struct Sender {
    pub inner: Option<mpsc::UnboundedSender<Command>>,
}

impl Sender {
    pub fn new() -> Self {
        Self { inner: None }
    }
}

impl Sender {}
