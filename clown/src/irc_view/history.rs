#[derive(Default)]
pub struct InputHistory {
    messages: Vec<String>,
    index: usize,

    saved_message: Option<String>,
}

//"a", "b", "c"
// When pushed at the back, the new index is the size - 1
// When going up, go to index-1 max is 0
impl InputHistory {
    pub fn add_message(&mut self, message: String) {
        self.messages.push(message);
        self.index = self.messages.len();
    }

    fn is_at_start(&self) -> bool {
        self.index == self.messages.len()
    }

    pub fn up(&mut self, previous_message: &str) {
        if self.is_at_start() {
            self.saved_message = Some(previous_message.to_string())
        }
        self.index = self.index.saturating_sub(1);
    }

    pub fn down(&mut self) {
        self.index = self.index.saturating_add(1).min(self.messages.len());
    }

    pub fn get_message(&self) -> Option<&str> {
        if self.is_at_start() {
            self.saved_message.as_deref()
        } else {
            self.messages.get(self.index).map(|v| v.as_str())
        }
    }
}
