#[derive(Default)]
pub struct InputHistory {
    messages: Vec<String>,
    index: usize,
}

//"a", "b", "c"
// When pushed at the back, the new index is the size - 1
// When going up, go to index-1 max is 0
impl InputHistory {
    pub fn new() -> Self {
        Self {
            messages: Vec::new(),
            index: 0,
        }
    }

    pub fn add_message(&mut self, message: String) {
        self.messages.push(message);
        self.index = self.messages.len();
    }

    pub fn up(&mut self) {
        self.index = self.index.saturating_sub(1);
    }

    pub fn down(&mut self) {
        self.index = self.index.saturating_add(1).min(self.messages.len());
    }

    pub fn get_message(&self) -> Option<&str> {
        self.messages.get(self.index).map(|v| v.as_str())
    }
}
