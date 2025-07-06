use tokio::sync::mpsc;
pub struct CommandReceiver {
    pub inner: mpsc::UnboundedReceiver<Command>,
}

pub struct Command {
    command_name: String,
    parameters: Vec<String>,
}

impl Command {
    pub fn as_bytes(&self) -> Vec<u8> {
        format!("{} {}", self.command_name, self.parameters.join(" "))
            .as_bytes()
            .to_vec()
    }

    pub fn nick() -> Self {
        Self {
            command_name: "NICK".to_string(),
            parameters: vec![],
        }
    }
}
