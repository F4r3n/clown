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
    CAP(String),

    /*Error code */
    //001
    WELCOME(String /*target */, String /*message */),
}

impl Command {
    pub fn as_bytes(&self) -> Vec<u8> {
        match &self {
            Command::PRIVMSG(channel, message) => format!("PRIVMSG {channel} :{message}\r\n")
                .as_bytes()
                .to_vec(),
            Command::NICK(target) => format!("NICK {target}\r\n").as_bytes().to_vec(),
            Command::CAP(content) => format! {"CAP {content}\r\n"}.as_bytes().to_vec(),
            Command::PASS(password) => format!("PASS {password}\r\n").as_bytes().to_vec(),
            Command::USER(username, realname) => format!("USER {username} 0 * :{realname}\r\n")
                .as_bytes()
                .to_vec(),
            _ => panic!("Not a supported command"),
        }
    }
}
