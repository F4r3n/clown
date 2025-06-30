pub enum Command {
    Connect,
    Quit,
}

pub fn parse_command(in_content: &str) -> Option<Command> {
    if let Some(next) = in_content.trim().to_ascii_lowercase().strip_prefix('/') {
        match next {
            "connect" => Some(Command::Connect),
            "quit" => Some(Command::Quit),
            _ => None,
        }
    } else {
        None
    }
}
