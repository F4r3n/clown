use crate::command::Command;
use crate::command::CommandReceiver;
use crate::error::IRCIOError;
use crate::message::{MessageReceiver, MessageSender, ServerMessage};
use crate::response::Response;
use clown_parser::message::create_message;

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
    ) -> Result<(), IRCIOError>
    where
        W: AsyncWrite + Unpin,
    {
        match server_message.get_reply() {
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

        if let Some(sender) = &self.message_sender {
            sender
                .inner
                .send(server_message)
                .map_err(|_| IRCIOError::SendMessage)?;
        }
        Ok(())
    }

    pub async fn process<R, W>(
        &mut self,
        mut reader: BufReader<R>,
        mut writer: BufWriter<W>,
    ) -> Result<(), IRCIOError>
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


                            if let Ok(message) = create_message(line.as_bytes())
                            {
                                let server_message = ServerMessage::new(message);
                                self.receive_message(&mut writer, server_message).await?;
                            }

                        }
                        Err(e) => {
                            return Err(IRCIOError::IO(e));
                        }
                    }
                }
                command = match self.receiver.as_mut() {
                    Some(receiver) => receiver.inner.recv(),
                    None => break, // Receiver not set, break the loop
                } => {
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
    pub fn send(&mut self, in_command: Command) -> Result<(), IRCIOError> {
        if let Some(inner) = &self.inner {
            inner
                .send(in_command)
                .map_err(|_| IRCIOError::SendCommand)?
        }
        Ok(())
    }

    pub fn is_closed(&self) -> bool {
        if let Some(inner) = &self.inner {
            inner.is_closed()
        } else {
            true
        }
    }
}
