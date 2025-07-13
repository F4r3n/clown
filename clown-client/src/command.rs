pub enum ClientCommand {
    Connect,
    Join,
    Quit,
}

pub fn parse_command(in_content: &str) -> Option<ClientCommand> {
    if let Some(next) = in_content.trim().to_ascii_lowercase().strip_prefix('/') {
        match next {
            "connect" => Some(ClientCommand::Connect),
            "quit" => Some(ClientCommand::Quit),
            "join" => Some(ClientCommand::Join),
            _ => None,
        }
    } else {
        None
    }
}
