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
        mut writer: &mut BufWriter<W>,
        server_message: ServerMessage,
    ) -> Result<(), IRCIOError>
    where
        W: AsyncWrite + Unpin,
    {
        match server_message.reply() {
            Response::Cmd(Command::Ping(token)) => {
                Command::Pong(token).write(&mut writer).await?;
                writer.flush().await?;
            }
            Response::Cmd(Command::Cap(_)) => {
                Command::Cap("END".into()).write(&mut writer).await?;
                writer.flush().await?;
            }
            _ => {}
        }

        if let Some(sender) = &self.message_sender {
            sender
                .inner
                .send(server_message)
                .await
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
        let mut receiver = self.receiver.take().ok_or(IRCIOError::Uninitialized)?;
        loop {
            tokio::select! {
                response = reader.read_line(&mut buffer) => {
                    match response {
                        Ok(0) => break, // Connection closed
                        Ok(_) => {
                            let line = buffer.trim_end();
                            if let Ok(message) = create_message(line.as_bytes())
                            {
                                let server_message = ServerMessage::new(message);
                                self.receive_message(&mut writer, server_message).await?;
                            }
                            buffer.clear();
                        }
                        Err(e) => {
                            return Err(IRCIOError::IO(e));
                        }
                    }
                }
                cmd = receiver.inner.recv() => {
                    match cmd {
                        Some(cmd)=> {
                            cmd.write(&mut writer).await?;
                            writer.flush().await?;
                        }
                        None => break
                    }
                }
            }
        }
        Ok(())
    }

    pub fn create_outgoing(&mut self) -> (CommandSender, MessageReceiver) {
        let (command_sender, command_receiver) = mpsc::unbounded_channel::<Command>();
        let (message_sender, message_receiver) = mpsc::channel::<ServerMessage>(100);
        self.receiver = Some(CommandReceiver {
            inner: command_receiver,
        });
        self.message_sender = Some(MessageSender {
            inner: message_sender,
        });
        (
            CommandSender {
                inner: command_sender,
            },
            MessageReceiver {
                inner: message_receiver,
            },
        )
    }
}

#[derive(Clone)]
pub struct CommandSender {
    pub inner: mpsc::UnboundedSender<Command>,
}

impl CommandSender {
    pub fn send(&mut self, in_command: Command) -> Result<(), IRCIOError> {
        self.inner
            .send(in_command)
            .map_err(|_| IRCIOError::SendCommand)
    }

    pub fn is_closed(&self) -> bool {
        self.inner.is_closed()
    }
}
