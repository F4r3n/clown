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
        self.saved_message = None;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_and_navigation() {
        let mut history = InputHistory::default();
        history.add_message("first".to_string());
        history.add_message("second".to_string());

        // Initial state: index is at 2 (the live prompt)
        assert_eq!(history.get_message(), None);

        // Move up to "second"
        history.up("");
        assert_eq!(history.get_message(), Some("second"));

        // Move up to "first"
        history.up("second");
        assert_eq!(history.get_message(), Some("first"));

        // Boundary: Move up again (should stay at "first")
        history.up("first");
        assert_eq!(history.get_message(), Some("first"));

        // Move back down to "second"
        history.down();
        assert_eq!(history.get_message(), Some("second"));

        // Move back down to live prompt
        history.down();
        assert_eq!(history.get_message(), Some(""));
    }

    #[test]
    fn test_saved_message_persistence() {
        let mut history = InputHistory::default();
        history.add_message("old command".to_string());

        // User typed "half finished command" but then pressed Up
        history.up("half finished command");
        assert_eq!(history.get_message(), Some("old command"));

        // User presses Down to return to their unfinished work
        history.down();
        assert_eq!(history.get_message(), Some("half finished command"));
    }

    #[test]
    fn test_empty_history() {
        let mut history = InputHistory::default();

        // Pressing up/down on empty history shouldn't crash
        history.up("nothing");
        assert_eq!(history.get_message(), Some("nothing"));

        history.down();
        assert_eq!(history.get_message(), Some("nothing"));
    }

    #[test]
    fn test_index_reset_after_add() {
        let mut history = InputHistory::default();
        history.add_message("one".to_string());

        // Go back in history
        history.up("");
        assert_eq!(history.get_message(), Some("one"));

        // Adding a new message should reset the index to the end
        history.add_message("two".to_string());
        assert_eq!(history.get_message(), None);

        // Verify we can now see "two" by going up
        history.up("");
        assert_eq!(history.get_message(), Some("two"));
    }
}
