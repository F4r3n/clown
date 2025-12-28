use clown_parser::message::Message;
use tokio::sync::mpsc;

use crate::{
    command::CommandBuilder,
    response::{Response, ResponseBuilder},
};
pub struct MessageSender {
    pub inner: mpsc::Sender<ServerMessage>,
}

pub struct MessageReceiver {
    pub inner: mpsc::Receiver<ServerMessage>,
}

#[derive(Debug)]
pub struct ServerMessage {
    message: Message,
}

impl ServerMessage {
    pub fn new(message: Message) -> Self {
        Self { message }
    }

    pub fn source(&self) -> Option<&str> {
        self.message.source()
    }

    pub fn reply(&self) -> Response {
        if let Some(command) = self.message.command_name() {
            let params = self.message.parameters().collect::<Vec<&str>>();

            if let Ok(command_number) = command.parse() {
                Response::Rpl(ResponseBuilder::get_reply(
                    command_number,
                    &params,
                    self.message.trailing(),
                ))
            } else {
                CommandBuilder::get_command(command, &params, self.message.trailing())
                    .map(Response::Cmd)
                    .unwrap_or(Response::Unknown(format!("{:?}", self.message)))
            }
        } else {
            Response::Unknown(format!("{:?}", self.message))
        }
    }
}

#[cfg(test)]
mod tests {
    use clown_parser::message::create_message;

    use crate::{
        command::Command,
        message::ServerMessage,
        response::{Response, ResponseNumber},
    };

    #[test]
    fn test_privmsg() {
        let message =
            create_message(":Angel PRIVMSG Wiz :Hello are you receiving this message ?".as_bytes())
                .unwrap();
        let server_message = ServerMessage::new(message);
        let command = server_message.reply();
        if let Response::Cmd(crate::command::Command::PrivMsg(target, message)) = command {
            assert_eq!(target, "Wiz", "PRIVMSG target mismatch");
            assert_eq!(
                message, "Hello are you receiving this message ?",
                "PRIVMSG message mismatch"
            );
        } else {
            panic!("Expected PRIVMSG command, got {command:?}");
        }
    }

    #[test]
    fn test_quit() {
        let message = create_message(":Alice QUIT :Quit: Leaving\r\n".as_bytes()).unwrap();
        let server_message = ServerMessage::new(message);
        let command = server_message.reply();
        if let Response::Cmd(crate::command::Command::Quit(reason)) = command {
            assert_eq!(
                reason,
                Some("Quit: Leaving".to_string()),
                "QUIT reason mismatch"
            );
        } else {
            panic!("Expected QUIT command, got {command:?}");
        }
    }

    #[test]
    fn test_quit_no_reason() {
        let message = create_message(":Alice QUIT\r\n".as_bytes()).unwrap();
        let server_message = ServerMessage::new(message);
        let command = server_message.reply();
        if let Response::Cmd(crate::command::Command::Quit(reason)) = command {
            assert_eq!(reason, None, "QUIT reason mismatch");
        } else {
            panic!("Expected QUIT command, got {command:?}");
        }
    }

    #[test]
    fn test_ping() {
        let message = create_message(":token PING :token\r\n".as_bytes()).unwrap();
        let server_message = ServerMessage::new(message);
        let command = server_message.reply();
        if let Response::Cmd(crate::command::Command::Ping(token)) = command {
            assert_eq!(token, "token", "PING token mismatch");
        } else {
            panic!("Expected PING command, got {command:?}");
        }
    }

    #[test]
    fn test_pong() {
        let message = create_message("PONG serverName 123456789\r\n".as_bytes()).unwrap();
        let server_message = ServerMessage::new(message);
        let command = server_message.reply();
        if let Response::Cmd(Command::Pong(token)) = command {
            assert_eq!(token, "123456789", "PONG token mismatch");
        } else {
            panic!("Expected PONG command, got {command:?}");
        }
    }

    #[test]
    fn test_nick_trailing() {
        let message = create_message(":test!farine4@inspircd NICK :jo\r\n".as_bytes()).unwrap();
        let server_message = ServerMessage::new(message);
        assert_eq!(
            server_message.source(),
            Some("test"),
            "{:?}",
            server_message.source()
        );
        let command = server_message.reply();

        if let Response::Cmd(Command::Nick(new_name)) = command {
            assert_eq!(new_name, "jo", "NICK token mismatch");
        } else {
            panic!("Expected NICK command, got {command:?}");
        }
    }

    #[test]
    fn test_nick_params() {
        let message = create_message(":test!farine4@inspircd NICK jo\r\n".as_bytes()).unwrap();
        let server_message = ServerMessage::new(message);
        assert_eq!(
            server_message.source(),
            Some("test"),
            "{:?}",
            server_message.source()
        );
        let command = server_message.reply();

        if let Response::Cmd(Command::Nick(new_name)) = command {
            assert_eq!(new_name, "jo", "NICK token mismatch");
        } else {
            panic!("Expected NICK command, got {command:?}");
        }
    }

    #[test]
    fn test_unknown_numeric_reply() {
        let message = create_message(b":irc.example.com 999 Nick :Unknown numeric reply").unwrap();
        let server_message = ServerMessage::new(message);
        let reply = server_message.reply();
        assert!(
            matches!(reply, Response::Rpl(ResponseNumber::Unknown(999, msg)) if msg == "Unknown numeric reply")
        );
    }

    #[test]
    fn test_notice_looking_up_hostname() {
        let message =
            create_message(b":irc.example.net NOTICE * :*** Looking up your hostname...").unwrap();
        let server_message = ServerMessage::new(message);
        let reply = server_message.reply();
        assert!(
            matches!(reply, Response::Cmd(Command::Notice(target, msg)) if target == "*" && msg == "*** Looking up your hostname...")
        );
    }

    #[test]
    fn test_notice_found_hostname() {
        let message =
            create_message(b":irc.example.net NOTICE farine :*** Found your hostname (inspircd)")
                .unwrap();
        let server_message = ServerMessage::new(message);
        let reply = server_message.reply();
        if let Response::Cmd(Command::Notice(target, msg)) = reply {
            assert_eq!(target, "farine");
            assert_eq!(msg, "*** Found your hostname (inspircd)");
        } else {
            panic!("Not notice");
        }
    }

    #[test]
    fn test_numeric_reply_001() {
        let message = create_message(b":irc.example.net 001 farine :Welcome to the ExampleNet IRC Network farine!farine@inspircd").unwrap();
        let server_message = ServerMessage::new(message);
        let reply = server_message.reply();

        assert!(
            matches!(reply, Response::Rpl(ResponseNumber::Welcome(ref msg)) if msg == "Welcome to the ExampleNet IRC Network farine!farine@inspircd")
        );
    }

    #[test]
    fn test_numeric_reply_265() {
        let message =
            create_message(b":irc.example.net 265 farine :Current local users: 0  Max: 0").unwrap();
        let server_message = ServerMessage::new(message);
        let reply = server_message.reply();
        assert!(
            matches!(reply, Response::Rpl(ResponseNumber::LocalUsers(msg))
            if msg == "Current local users: 0  Max: 0")
        );
    }

    #[test]
    fn test_empty_command() {
        let message = create_message(b":irc.example.com").unwrap();
        let server_message = ServerMessage::new(message);
        let reply = server_message.reply();

        assert!(matches!(reply, Response::Unknown { .. }));
    }

    #[test]
    fn test_topic_333_command() {
        let message =
            create_message(b":IRC-server 333 farine_test #rust-spam farineA 1754165495").unwrap();
        let server_message = ServerMessage::new(message);
        let reply = server_message.reply();
        assert!(
            matches!(reply, Response::Rpl(ResponseNumber::TopicWhoTime(msg))
            if msg == "farine_test #rust-spam farineA 1754165495")
        );
    }

    #[test]
    fn test_topic_command() {
        let message = create_message(b":farineA!farine4@hidden TOPIC #rust-spam :yo").unwrap();
        let server_message = ServerMessage::new(message);
        let reply = server_message.reply();
        assert!(matches!(reply, Response::Cmd(Command::Topic(channel, msg))
            if msg == "yo" && channel == "#rust-spam"));
    }
}
