use tokio::sync::mpsc;
pub struct CommandReceiver {
    pub inner: mpsc::UnboundedReceiver<Command>,
}
#[derive(Debug)]
pub enum Command {
    NICK(String /*target */),
    PRIVMSG(String /*channel */, String /*message */),
    PASS(String /*password */),
    USER(String /*username */, String /*realname */),
    PING(String /*token */),
    PONG(String /*token */),
    QUIT(Option<String>),

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
            _ => panic!("Not a supported command"),
        }
    }
}
