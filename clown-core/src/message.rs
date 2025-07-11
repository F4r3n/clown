use clown_parser::message::Message;
use tokio::sync::mpsc;

use crate::command::{Command, CommandBuilder};
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
    pub fn get_command(&self) -> Option<Command> {
        if let Some(command) = self.message.get_command_name() {
            CommandBuilder::get_command(command, self.message.get_trailling())
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
        let command = server_message.get_command();
        assert!(command.is_some());
        if let Some(crate::command::Command::PRIVMSG(target, message)) = command {
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
    fn test_welcome() -> anyhow::Result<()> {
        let message = create_message(
            ":irc.freenode.net 001 Bob :Welcome to the Internet Relay Chat Network Bob!~bob@192.0.2.1\r\n".as_bytes(),
        )?;
        let server_message = ServerMessage::new(message);
        let command = server_message.get_command();
        assert!(command.is_some());
        if let Some(crate::command::Command::WELCOME(target, message)) = command {
            assert_eq!(target, "Bob", "WELCOME target mismatch");
            assert_eq!(
                message,
                "Welcome to the Internet Relay Chat Network Bob!~bob@192.0.2.1"
            );
        } else {
            panic!("Expected WELCOME command, got {command:?}");
        }

        Ok(())
    }

    #[test]
    fn test_quit() -> anyhow::Result<()> {
        let message = create_message(":Alice QUIT :Quit: Leaving\r\n".as_bytes())?;
        let server_message = ServerMessage::new(message);
        let command = server_message.get_command();
        assert!(command.is_some());
        if let Some(crate::command::Command::QUIT(reason)) = command {
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
        let command = server_message.get_command();
        assert!(command.is_some());
        if let Some(crate::command::Command::QUIT(reason)) = command {
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
        let command = server_message.get_command();
        assert!(command.is_some());
        if let Some(crate::command::Command::PING(token)) = command {
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
        let command = server_message.get_command();
        assert!(command.is_some());
        if let Some(crate::command::Command::PONG(token)) = command {
            assert_eq!(token, "123456789", "PONG token mismatch");
        } else {
            panic!("Expected PONG command, got {command:?}");
        }
        Ok(())
    }
}
