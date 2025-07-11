use clown_parser::message::Message;
use tokio::sync::mpsc;

use crate::{
    command::CommandBuilder,
    reply::{Reply, ReplyBuilder, ReplyNumber},
};
pub struct MessageSender {
    pub inner: mpsc::UnboundedSender<ServerMessage>,
}

pub struct MessageReceiver {
    pub inner: mpsc::UnboundedReceiver<ServerMessage>,
}

pub struct ServerMessage {
    message: Message,
}

impl ServerMessage {
    pub fn new(message: Message) -> Self {
        Self { message }
    }
    pub fn get_reply(&self) -> Option<Reply> {
        if let Some(command) = self.message.get_command_name() {
            if let Ok(command_number) = command.parse() {
                ReplyBuilder::get_reply(command_number, self.message.get_trailling())
                    .map(Reply::Rpl)
            } else {
                CommandBuilder::get_command(command, self.message.get_trailling()).map(Reply::Cmd)
            }
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use clown_parser::message::create_message;

    use crate::message::ServerMessage;

    #[test]
    fn test_privmsg() -> anyhow::Result<()> {
        let message = create_message(
            ":Angel PRIVMSG Wiz :Hello are you receiving this message ?".as_bytes(),
        )?;
        let server_message = ServerMessage::new(message);
        let command = server_message.get_reply();
        assert!(command.is_some());
        if let Some(crate::reply::Reply::Cmd(crate::command::Command::PrivMsg(target, message))) =
            command
        {
            assert_eq!(target, "Wiz", "PRIVMSG target mismatch");
            assert_eq!(
                message, "Hello are you receiving this message ?",
                "PRIVMSG message mismatch"
            );
        } else {
            panic!("Expected PRIVMSG command, got {command:?}");
        }

        Ok(())
    }

    #[test]
    fn test_quit() -> anyhow::Result<()> {
        let message = create_message(":Alice QUIT :Quit: Leaving\r\n".as_bytes())?;
        let server_message = ServerMessage::new(message);
        let command = server_message.get_reply();
        assert!(command.is_some());
        if let Some(crate::reply::Reply::Cmd(crate::command::Command::Quit(reason))) = command {
            assert_eq!(
                reason,
                Some("Quit: Leaving".to_string()),
                "QUIT reason mismatch"
            );
        } else {
            panic!("Expected QUIT command, got {command:?}");
        }
        Ok(())
    }

    #[test]
    fn test_quit_no_reason() -> anyhow::Result<()> {
        let message = create_message(":Alice QUIT\r\n".as_bytes())?;
        let server_message = ServerMessage::new(message);
        let command = server_message.get_reply();
        assert!(command.is_some());
        if let Some(crate::reply::Reply::Cmd(crate::command::Command::Quit(reason))) = command {
            assert_eq!(reason, None, "QUIT reason mismatch");
        } else {
            panic!("Expected QUIT command, got {command:?}");
        }
        Ok(())
    }

    #[test]
    fn test_ping() -> anyhow::Result<()> {
        let message = create_message("PING :123456789\r\n".as_bytes())?;
        let server_message = ServerMessage::new(message);
        let command = server_message.get_reply();
        assert!(command.is_some());
        if let Some(crate::reply::Reply::Cmd(crate::command::Command::Ping(token))) = command {
            assert_eq!(token, "123456789", "PING token mismatch");
        } else {
            panic!("Expected PING command, got {command:?}");
        }
        Ok(())
    }

    #[test]
    fn test_pong() -> anyhow::Result<()> {
        let message = create_message("PONG serverName :123456789\r\n".as_bytes())?;
        let server_message = ServerMessage::new(message);
        let command = server_message.get_reply();
        assert!(command.is_some());
        if let Some(crate::reply::Reply::Cmd(crate::command::Command::Pong(token))) = command {
            assert_eq!(token, "123456789", "PONG token mismatch");
        } else {
            panic!("Expected PONG command, got {command:?}");
        }
        Ok(())
    }
}
