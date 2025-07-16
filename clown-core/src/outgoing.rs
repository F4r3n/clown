use crate::command::Command;
use crate::command::CommandReceiver;
use crate::message::{MessageReceiver, MessageSender, ServerMessage};
use crate::response::Response;
use clown_parser::message::create_message;
use std::fs::File;
use std::io::Write;
use tokio::io::BufReader;
use tokio::io::{AsyncBufReadExt, AsyncRead};
use tokio::io::{AsyncWrite, AsyncWriteExt, BufWriter};
use tokio::sync::mpsc;
#[derive(Default)]
pub struct Outgoing {
    receiver: Option<CommandReceiver>,
    message_sender: Option<MessageSender>,
}

impl Outgoing {
    pub async fn receive_message<W>(
        &mut self,
        writer: &mut BufWriter<W>,
        server_message: ServerMessage,
    ) -> anyhow::Result<()>
    where
        W: AsyncWrite + Unpin,
    {
        // Handle PING immediately
        /*
        if line.starts_with("PING") {
            let response = line.replacen("PING", "PONG", 1);
            writer.write_all(response.as_bytes()).await?;
            writer.write_all(b"\r\n").await?;
            writer.flush().await?;
        } else if line.starts_with("CAP") {
            writer
                .write_all(Command::Cap("END".to_string()).as_bytes().as_slice())
                .await?;
            writer.flush().await?;
        }
        */
        if let Some(reply) = server_message.get_reply() {
            match reply {
                Response::Cmd(Command::Ping(token)) => {
                    writer
                        .write_all(Command::Pong(token).as_bytes().as_slice())
                        .await?;
                    writer.flush().await?;
                }
                Response::Cmd(Command::Cap(_)) => {
                    writer
                        .write_all(Command::Cap("END".into()).as_bytes().as_slice())
                        .await?;
                    writer.flush().await?;
                }
                _ => {}
            }
        }

        if let Some(sender) = &self.message_sender {
            sender.inner.send(server_message)?;
        }
        Ok(())
    }

    pub async fn process<R, W>(
        &mut self,
        mut log_writer: Option<std::io::BufWriter<File>>,
        mut reader: BufReader<R>,
        mut writer: BufWriter<W>,
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

                            if let Some(log) = log_writer.as_mut() {
                                writeln!(log, "{}", line.clone())?;
                                log.flush()?;
                            }
                            if let Ok(message) = create_message(line.as_bytes())
                            {
                                let server_message = ServerMessage::new(message);
                                self.receive_message(&mut writer, server_message).await?;
                            }

                        }
                        Err(e) => {
                            eprintln!("Read error: {e}");
                            break;
                        }
                    }
                }
                command = self.receiver.as_mut().unwrap().inner.recv() => {
                    if let Some(cmd) = command {
                        if let Some(log) = log_writer.as_mut() {
                            if let Ok( string )= std::str::from_utf8(cmd.as_bytes().as_slice()) {
                            write!(log, "{string}")?;
                            log.flush()?;

                            }
                        }
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

    pub fn create_outgoing(&mut self) -> (CommandSender, MessageReceiver) {
        let (command_sender, command_receiver) = mpsc::unbounded_channel::<Command>();
        let (message_sender, message_receiver) = mpsc::unbounded_channel::<ServerMessage>();
        self.receiver = Some(CommandReceiver {
            inner: command_receiver,
        });
        self.message_sender = Some(MessageSender {
            inner: message_sender,
        });
        (
            CommandSender {
                inner: Some(command_sender),
            },
            MessageReceiver {
                inner: message_receiver,
            },
        )
    }
}

#[derive(Clone, Default)]
pub struct CommandSender {
    pub inner: Option<mpsc::UnboundedSender<Command>>,
}

impl CommandSender {
    pub fn send(&mut self, in_command: Command) -> Result<(), anyhow::Error> {
        if let Some(inner) = &self.inner {
            inner.send(in_command)?
        }
        Ok(())
    }
}
