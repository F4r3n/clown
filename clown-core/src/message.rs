use clown_parser::message::Message;
use tokio::sync::mpsc;

use crate::{
    command::CommandBuilder,
    response::{Response, ResponseBuilder},
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
    pub fn get_reply(&self) -> Option<Response> {
        if let Some(command) = self.message.get_command_name() {
            if let Ok(command_number) = command.parse() {
                ResponseBuilder::get_reply(command_number, self.message.get_trailing())
                    .map(Response::Rpl)
            } else {
                CommandBuilder::get_command(
                    command,
                    self.message.get_parameters(),
                    self.message.get_trailing(),
                )
                .map(Response::Cmd)
            }
        } else {
            None
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
    fn test_privmsg() -> anyhow::Result<()> {
        let message = create_message(
            ":Angel PRIVMSG Wiz :Hello are you receiving this message ?".as_bytes(),
        )?;
        let server_message = ServerMessage::new(message);
        let command = server_message.get_reply();
        assert!(command.is_some());
        if let Some(Response::Cmd(crate::command::Command::PrivMsg(target, message))) = command {
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
        if let Some(Response::Cmd(crate::command::Command::Quit(reason))) = command {
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
        if let Some(Response::Cmd(crate::command::Command::Quit(reason))) = command {
            assert_eq!(reason, None, "QUIT reason mismatch");
        } else {
            panic!("Expected QUIT command, got {command:?}");
        }
        Ok(())
    }

    #[test]
    fn test_ping() -> anyhow::Result<()> {
        let message = create_message("PING token\r\n".as_bytes())?;
        let server_message = ServerMessage::new(message);
        let command = server_message.get_reply();
        assert!(command.is_some());
        if let Some(Response::Cmd(crate::command::Command::Ping(token))) = command {
            assert_eq!(token, "token", "PING token mismatch");
        } else {
            panic!("Expected PING command, got {command:?}");
        }
        Ok(())
    }

    #[test]
    fn test_pong() -> anyhow::Result<()> {
        let message = create_message("PONG serverName 123456789\r\n".as_bytes())?;
        let server_message = ServerMessage::new(message);
        let command = server_message.get_reply();
        assert!(command.is_some());
        if let Some(Response::Cmd(Command::Pong(token))) = command {
            assert_eq!(token, "123456789", "PONG token mismatch");
        } else {
            panic!("Expected PONG command, got {command:?}");
        }
        Ok(())
    }

    #[test]
    fn test_unknown_numeric_reply() -> anyhow::Result<()> {
        let message = create_message(b":irc.example.com 999 Nick :Unknown numeric reply")?;
        let server_message = ServerMessage::new(message);
        let reply = server_message.get_reply();
        assert!(
            matches!(reply, Some(Response::Rpl(ResponseNumber::Unknown(999, msg))) if msg == "Unknown numeric reply")
        );
        Ok(())
    }

    #[test]
    fn test_notice_looking_up_hostname() -> anyhow::Result<()> {
        let message =
            create_message(b":irc.example.net NOTICE * :*** Looking up your hostname...")?;
        let server_message = ServerMessage::new(message);
        let reply = server_message.get_reply();
        assert!(
            matches!(reply, Some(Response::Cmd(Command::Notice(target, msg))) if target == "*" && msg == "*** Looking up your hostname...")
        );
        Ok(())
    }

    #[test]
    fn test_notice_found_hostname() -> anyhow::Result<()> {
        let message =
            create_message(b":irc.example.net NOTICE farine :*** Found your hostname (inspircd)")?;
        let server_message = ServerMessage::new(message);
        let reply = server_message.get_reply();
        if let Some(Response::Cmd(Command::Notice(target, msg))) = reply {
            assert_eq!(target, "farine");
            assert_eq!(msg, "*** Found your hostname (inspircd)");
        } else {
            panic!("Not notice");
        }

        Ok(())
    }

    #[test]
    fn test_numeric_reply_001() -> anyhow::Result<()> {
        let message = create_message(b":irc.example.net 001 farine :Welcome to the ExampleNet IRC Network farine!farine@inspircd")?;
        let server_message = ServerMessage::new(message);
        let reply = server_message.get_reply();

        assert!(
            matches!(reply, Some(Response::Rpl(ResponseNumber::Welcome(msg))) if msg == "Welcome to the ExampleNet IRC Network farine!farine@inspircd")
        );
        Ok(())
    }

    #[test]
    fn test_numeric_reply_002() -> anyhow::Result<()> {
        let message = create_message(b":irc.example.net 002 farine :Your host is irc.example.net, running version InspIRCd-4")?;
        let server_message = ServerMessage::new(message);
        let reply = server_message.get_reply();
        assert!(
            matches!(reply, Some(Response::Rpl(ResponseNumber::YourHost(msg))) if msg == "Your host is irc.example.net, running version InspIRCd-4")
        );
        Ok(())
    }

    #[test]
    fn test_numeric_reply_003() -> anyhow::Result<()> {
        let message = create_message(
            b":irc.example.net 003 farine :This server was created on 12 Jul 2025 at 06:41:59 UTC",
        )?;
        let server_message = ServerMessage::new(message);
        let reply = server_message.get_reply();
        assert!(
            matches!(reply, Some(Response::Rpl(ResponseNumber::Created(msg))) if msg == "This server was created on 12 Jul 2025 at 06:41:59 UTC")
        );
        Ok(())
    }

    #[test]
    fn test_numeric_reply_004() -> anyhow::Result<()> {
        let message = create_message(
            b":irc.example.net 004 farine irc.example.net InspIRCd-4 iosw biklmnopstv :bklov",
        )?;
        let server_message = ServerMessage::new(message);
        let reply = server_message.get_reply();
        assert!(
            matches!(reply, Some(Response::Rpl(ResponseNumber::MyInfo(msg))) if msg == "bklov")
        );
        Ok(())
    }

    #[test]
    fn test_numeric_reply_005_1() -> anyhow::Result<()> {
        let message = create_message(b":irc.example.net 005 farine AWAYLEN=200 CASEMAPPING=ascii CHANLIMIT=#:20 CHANMODES=b,k,l,imnpst CHANNELLEN=60 CHANTYPES=# ELIST=CMNTU EXTBAN=, HOSTLEN=64 KEYLEN=32 KICKLEN=300 LINELEN=512 :are supported by this server")?;
        let server_message = ServerMessage::new(message);
        if let Some(Response::Rpl(ResponseNumber::Bounce(msg))) = server_message.get_reply() {
            assert_eq!(msg, "are supported by this server");
        } else {
            panic!("Wrong reply")
        }

        Ok(())
    }

    #[test]
    fn test_numeric_reply_005_2() -> anyhow::Result<()> {
        let message = create_message(b":irc.example.net 005 farine MAXLIST=b:100 MAXTARGETS=5 MODES=20 NAMELEN=130 NETWORK=ExampleNet NICKLEN=30 PREFIX=(ov)@+ SAFELIST SAFERATE STATUSMSG=@+ TOPICLEN=330 USERLEN=10 :are supported by this server")?;
        let server_message = ServerMessage::new(message);
        let reply = server_message.get_reply();
        assert!(
            matches!(reply, Some(Response::Rpl(ResponseNumber::Bounce(msg))) if msg == "are supported by this server")
        );
        Ok(())
    }

    #[test]
    fn test_numeric_reply_005_3() -> anyhow::Result<()> {
        let message = create_message(
            b":irc.example.net 005 farine USERMODES=,,s,iow WHOX :are supported by this server",
        )?;
        let server_message = ServerMessage::new(message);
        let reply = server_message.get_reply();
        assert!(
            matches!(reply, Some(Response::Rpl(ResponseNumber::Bounce(msg))) if msg == "are supported by this server")
        );
        Ok(())
    }

    #[test]
    fn test_numeric_reply_251() -> anyhow::Result<()> {
        let message = create_message(
            b":irc.example.net 251 farine :There are 0 users and 0 invisible on 1 servers",
        )?;
        let server_message = ServerMessage::new(message);
        let reply = server_message.get_reply();
        assert!(
            matches!(reply, Some(Response::Rpl(ResponseNumber::LUserClient(msg))) if msg == "There are 0 users and 0 invisible on 1 servers")
        );
        Ok(())
    }

    #[test]
    fn test_numeric_reply_253() -> anyhow::Result<()> {
        let message = create_message(b":irc.example.net 253 farine 1 :unknown connections")?;
        let server_message = ServerMessage::new(message);
        let reply = server_message.get_reply();
        assert!(
            matches!(reply, Some(Response::Rpl(ResponseNumber::LUserUnknown(msg))) if msg == "unknown connections")
        );
        Ok(())
    }

    #[test]
    fn test_numeric_reply_254() -> anyhow::Result<()> {
        let message = create_message(b":irc.example.net 254 farine 0 :channels formed")?;
        let server_message = ServerMessage::new(message);
        let reply = server_message.get_reply();
        assert!(
            matches!(reply, Some(Response::Rpl(ResponseNumber::LUserChannels(msg))) if msg == "channels formed")
        );
        Ok(())
    }

    #[test]
    fn test_numeric_reply_255() -> anyhow::Result<()> {
        let message =
            create_message(b":irc.example.net 255 farine :I have 0 clients and 0 servers")?;
        let server_message = ServerMessage::new(message);
        let reply = server_message.get_reply();
        assert!(
            matches!(reply, Some(Response::Rpl(ResponseNumber::LUserMe(msg))) if msg == "I have 0 clients and 0 servers")
        );
        Ok(())
    }

    #[test]
    fn test_numeric_reply_265() -> anyhow::Result<()> {
        let message =
            create_message(b":irc.example.net 265 farine :Current local users: 0  Max: 0")?;
        let server_message = ServerMessage::new(message);
        let reply = server_message.get_reply();
        assert!(
            matches!(reply, Some(Response::Rpl(ResponseNumber::LocalUsers(msg))) if msg == "Current local users: 0  Max: 0")
        );
        Ok(())
    }

    #[test]
    fn test_numeric_reply_266() -> anyhow::Result<()> {
        let message =
            create_message(b":irc.example.net 266 farine :Current global users: 0  Max: 0")?;
        let server_message = ServerMessage::new(message);
        let reply = server_message.get_reply();
        assert!(
            matches!(reply, Some(Response::Rpl(ResponseNumber::GlobalUsers(msg))) if msg == "Current global users: 0  Max: 0")
        );
        Ok(())
    }

    #[test]
    fn test_numeric_reply_250() -> anyhow::Result<()> {
        let message = create_message(b":irc.example.net 250 farine :Highest connection count: 0 (0 clients) (1 connections received)")?;
        let server_message = ServerMessage::new(message);
        let reply = server_message.get_reply();
        assert!(
            matches!(reply, Some(Response::Rpl(ResponseNumber::HighestConnCount(msg))) if msg == "Highest connection count: 0 (0 clients) (1 connections received)")
        );
        Ok(())
    }

    #[test]
    fn test_numeric_reply_422() -> anyhow::Result<()> {
        let message =
            create_message(b":irc.example.net 422 farine :There is no message of the day.")?;
        let server_message = ServerMessage::new(message);
        let reply = server_message.get_reply();
        assert!(
            matches!(reply, Some(Response::Rpl(ResponseNumber::Unknown(422, msg))) if msg == "There is no message of the day.")
        );
        Ok(())
    }

    #[test]
    fn test_empty_command() -> anyhow::Result<()> {
        let message = create_message(b":irc.example.com")?;
        let server_message = ServerMessage::new(message);
        let reply = server_message.get_reply();
        assert!(reply.is_none());
        Ok(())
    }
}
