use tokio::sync::mpsc;

pub struct CommandReceiver {
    pub inner: mpsc::UnboundedReceiver<Command>,
}
#[derive(Debug, Clone)]
pub enum Command {
    NICK(String /*target */),
    PRIVMSG(String /*channel */, String /*message */),
    PASS(String /*password */),
    USER(String /*username */, String /*realname */),
    PING(String /*token */),
    PONG(String /*token */),
    QUIT(Option<String>),
    JOIN(String),
    CAP(String),

    /*Error code */
    //001
    WELCOME(String /*target */, String /*message */),
}

impl Command {
    pub fn as_bytes(&self) -> Vec<u8> {
        match &self {
            Command::PRIVMSG(channel, message) => {
                format!("PRIVMSG {channel} :{message}\r\n").into_bytes()
            }
            Command::JOIN(channel) => format!("JOIN {channel}\r\n").into_bytes(),
            Command::NICK(target) => format!("NICK {target}\r\n").into_bytes(),
            Command::CAP(content) => format! {"CAP {content}\r\n"}.into_bytes(),
            Command::PASS(password) => format!("PASS {password}\r\n").into_bytes(),
            Command::USER(username, realname) => format!("USER {username} 0 * :{realname}\r\n")
                .as_bytes()
                .to_vec(),
            _ => panic!("Not a supported command"),
        }
    }
}
