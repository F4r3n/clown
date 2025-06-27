use crate::component::WidgetId;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct FocusManager<'a> {
    /// List of focusable widget IDs in order
    focusable_widgets: Vec<WidgetId<'a>>,
    /// Current focused widget index
    current_focus_id: Option<WidgetId<'a>>,
    /// Map of widget ID to whether it's currently visible/enabled
    widget_states: HashMap<WidgetId<'a>, bool>,
}
//ctrl+n ctrl+p
impl<'a> FocusManager<'a> {
    pub fn new() -> Self {
        Self {
            focusable_widgets: Vec::new(),
            current_focus_id: None,
            widget_states: HashMap::new(),
        }
    }

    pub fn register_widget(&mut self, widget_id: WidgetId<'a>) {
        if !self.focusable_widgets.contains(&widget_id) {
            self.focusable_widgets.push(widget_id);
            if self.current_focus_id.is_none() {
                self.current_focus_id = Some(widget_id);
            }
            self.widget_states.insert(widget_id, true);
        }
    }
    fn next_index(&self, current: usize, direction: i8) -> usize {
        let len = self.focusable_widgets.len();
        match direction {
            d if d > 0 => (current + 1) % len,
            d if d < 0 => (current + len - 1) % len,
            _ => current,
        }
    }
    fn get_next_focus(&self, in_id: &WidgetId<'a>, direction: i8) -> Option<usize> {
        let len = self.focusable_widgets.len();
        if len == 0 {
            return None;
        }
        if let Some(mut current) = self.focusable_widgets.iter().position(|id| *id == *in_id) {
            let start_pos = current;
            current = self.next_index(current, direction);
            loop {
                if let Some(widget_id) = self.focusable_widgets.get(current) {
                    if let Some(is_valid) = self.widget_states.get(widget_id) {
                        if *is_valid {
                            return Some(current);
                        } else {
                            current = self.next_index(current, direction);
                        }
                    } else {
                        current = self.next_index(current, direction);
                    }
                }
                if start_pos == current {
                    return None;
                }
            }
        }
        return None;
    }

    fn get_next_focus_id(&self, in_id: &WidgetId<'a>, direction: i8) -> Option<&WidgetId<'a>> {
        if let Some(index) = self.get_next_focus(in_id, direction) {
            return self.focusable_widgets.get(index);
        }
        None
    }
    #[cfg(test)]
    pub fn unregister_widget(&mut self, widget_id: &WidgetId<'a>) {
        if let Some(pos) = self
            .focusable_widgets
            .iter()
            .position(|id| *id == *widget_id)
        {
            self.current_focus_id = self.get_next_focus_id(&widget_id, 1).cloned();

            self.focusable_widgets.remove(pos);
            self.widget_states.remove(widget_id);
        }
    }
    #[cfg(test)]
    pub fn set_widget_enabled(&mut self, widget_id: &WidgetId<'a>, enabled: bool) {
        self.widget_states.insert(widget_id, enabled);
    }

    pub fn get_focused_widget(&self) -> Option<&WidgetId<'a>> {
        return self.current_focus_id.as_ref();
    }
    #[cfg(test)]
    pub fn has_focus(&self, widget_id: &WidgetId<'a>) -> bool {
        if let Some(focused_widget) = self.get_focused_widget() {
            focused_widget == widget_id
        } else {
            false
        }
    }

    pub fn focus_next(&mut self) {
        if self.focusable_widgets.is_empty() {
            return;
        }
        if let Some(id) = &self.current_focus_id {
            self.current_focus_id = self.get_next_focus_id(&id, 1).cloned();
        }
    }

    pub fn focus_previous(&mut self) {
        if self.focusable_widgets.is_empty() {
            return;
        }

        if let Some(id) = &self.current_focus_id {
            self.current_focus_id = self.get_next_focus_id(&id, -1).cloned();
        }
    }
    #[cfg(test)]
    pub fn set_focus(&mut self, widget_id: &WidgetId<'a>) -> bool {
        if let Some(value) = self.widget_states.get(widget_id) {
            if *value {
                self.current_focus_id = Some(widget_id);
                return true;
            } else {
                return false;
            }
        }
        false
    }
    #[cfg(test)]
    pub fn get_all_widgets(&self) -> &Vec<WidgetId<'a>> {
        &self.focusable_widgets
    }
    #[cfg(test)]
    pub fn get_enabled_widgets(&self) -> Vec<&WidgetId<'a>> {
        self.focusable_widgets
            .iter()
            .filter(|id| *self.widget_states.get(*id).unwrap_or(&true))
            .collect()
    }
}

impl<'a> Default for FocusManager<'a> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_focus_manager_basic() {
        let mut fm = FocusManager::new();

        // Initially no widgets
        assert_eq!(fm.get_focused_widget(), None);

        // Register widgets
        let widget1 = "widget1";
        let widget2 = "widget1";

        fm.register_widget(widget1);
        fm.register_widget(widget2);

        // First widget should have focus
        assert_eq!(fm.get_focused_widget(), Some(&widget1));
        assert!(fm.has_focus(&widget1));
        assert!(!fm.has_focus(&widget2));

        // Move to next
        fm.focus_next();
        assert_eq!(fm.get_focused_widget(), Some(&widget2));
        assert!(!fm.has_focus(&widget1));
        assert!(fm.has_focus(&widget2));

        // Move to next (should wrap around)
        fm.focus_next();
        assert_eq!(fm.get_focused_widget(), Some(&widget1));
    }

    #[test]
    fn test_focus_manager_disabled_widgets() {
        let mut fm = FocusManager::new();
        let w1 = "widget1";
        let w2 = "widget2";
        let w3 = "widget3";

        fm.register_widget(w1);
        fm.register_widget(w2);
        fm.register_widget(w3);

        // Disable widget2
        fm.set_widget_enabled(&w2, false);

        // Should start with widget1
        assert_eq!(fm.get_focused_widget(), Some(&w1));

        // Next should skip widget2 and go to widget3
        fm.focus_next();
        assert_eq!(fm.get_focused_widget(), Some(&w3));

        // Previous should skip widget2 and go back to widget1
        fm.focus_previous();
        assert_eq!(fm.get_focused_widget(), Some(&w1));
    }

    #[test]
    fn test_set_focus() {
        let mut fm = FocusManager::new();
        let w1 = "widget1";
        let w2 = "widget2";
        let w3 = "widget3";
        let non_existent = "nonexistent";

        fm.register_widget(w1);
        fm.register_widget(w2);
        fm.register_widget(w3);

        // Set focus to widget3
        assert!(fm.set_focus(&w3));
        assert_eq!(fm.get_focused_widget(), Some(&w3));

        // Try to set focus to disabled widget
        fm.set_widget_enabled(&w2, false);
        assert!(!fm.set_focus(&w2));

        // Try to set focus to non-existent widget
        assert!(!fm.set_focus(&non_existent));
    }

    #[test]
    fn test_empty_focus_manager() {
        let mut fm = FocusManager::new();
        let any_widget = "any_widget";

        // Empty focus manager should have no focused widget
        assert_eq!(fm.get_focused_widget(), None);
        assert!(!fm.has_focus(&any_widget));

        // Moving focus on empty manager should not crash
        fm.focus_next();
        fm.focus_previous();
        assert_eq!(fm.get_focused_widget(), None);

        // Setting focus on non-existent widget should fail
        assert!(!fm.set_focus(&any_widget));
    }

    #[test]
    fn test_single_widget() {
        let mut fm = FocusManager::new();
        let only_widget = "only_widget";

        fm.register_widget(only_widget);

        // Single widget should be focused
        assert_eq!(fm.get_focused_widget(), Some(&only_widget));
        assert!(fm.has_focus(&only_widget));

        // Moving focus should stay on the same widget
        fm.focus_next();
        assert_eq!(fm.get_focused_widget(), Some(&only_widget));

        fm.focus_previous();
        assert_eq!(fm.get_focused_widget(), Some(&only_widget));
    }

    #[test]
    fn test_duplicate_widget_registration() {
        let mut fm = FocusManager::new();
        let w1 = "widget1";
        let w2 = "widget2";

        fm.register_widget(w1);
        fm.register_widget(w2);

        // Register the same widget again
        fm.register_widget(w1);

        // Should still only have 2 widgets
        assert_eq!(fm.get_all_widgets().len(), 2);
        assert_eq!(fm.get_all_widgets(), &vec![w1, w2]);
    }

    #[test]
    fn test_unregister_widget() {
        let mut fm = FocusManager::new();
        let w1 = "widget1";
        let w2 = "widget2";
        let w3 = "widget3";
        let non_existent = "nonexistent";

        fm.register_widget(w1);
        fm.register_widget(w2);
        fm.register_widget(w3);

        // Focus should be on widget1
        assert_eq!(fm.get_focused_widget(), Some(&w1));

        // Unregister the focused widget
        fm.unregister_widget(&w1);

        // Focus should move to next widget
        assert_eq!(fm.get_focused_widget(), Some(&w2));
        assert_eq!(fm.get_all_widgets().len(), 2);

        // Unregister non-existent widget should not crash
        fm.unregister_widget(&non_existent);
        assert_eq!(fm.get_all_widgets().len(), 2);
    }

    #[test]
    fn test_unregister_last_widget() {
        let mut fm = FocusManager::new();
        let only_widget = "only_widget";

        fm.register_widget(only_widget);
        assert_eq!(fm.get_focused_widget(), Some(&only_widget));

        // Unregister the only widget
        fm.unregister_widget(&only_widget);

        // Should have no focused widget
        assert_eq!(fm.get_focused_widget(), None);
        assert_eq!(fm.get_all_widgets().len(), 0);
    }

    #[test]
    fn test_all_widgets_disabled() {
        let mut fm = FocusManager::new();
        let w1 = "widget1";
        let w2 = "widget2";
        let w3 = "widget3";

        fm.register_widget(w1);
        fm.register_widget(w2);
        fm.register_widget(w3);

        // Disable all widgets
        fm.set_widget_enabled(&w1, false);
        fm.set_widget_enabled(&w2, false);
        fm.set_widget_enabled(&w3, false);

        // Focus navigation should not move focus when all widgets are disabled
        let initial_focus = fm.get_focused_widget().cloned();
        fm.focus_next();
        assert_eq!(fm.get_focused_widget(), initial_focus.as_ref());

        fm.focus_previous();
        assert_eq!(fm.get_focused_widget(), initial_focus.as_ref());

        // Setting focus to disabled widget should fail
        assert!(!fm.set_focus(&w1));
        assert!(!fm.set_focus(&w2));
        assert!(!fm.set_focus(&w3));
    }

    #[test]
    fn test_get_enabled_widgets() {
        let mut fm = FocusManager::new();
        let w1 = "widget1";
        let w2 = "widget2";
        let w3 = "widget3";

        fm.register_widget(w1);
        fm.register_widget(w2);
        fm.register_widget(w3);

        // All widgets should be enabled by default
        let enabled = fm.get_enabled_widgets();
        assert_eq!(enabled.len(), 3);

        // Disable one widget
        fm.set_widget_enabled(&w2, false);
        let enabled = fm.get_enabled_widgets();
        assert_eq!(enabled.len(), 2);
        assert!(enabled.contains(&&w1));
        assert!(enabled.contains(&&w3));
        assert!(!enabled.contains(&&w2));
    }

    #[test]
    fn test_focus_wrapping() {
        let mut fm = FocusManager::new();
        let w1 = "widget1";
        let w2 = "widget2";
        let w3 = "widget3";

        fm.register_widget(w1);
        fm.register_widget(w2);
        fm.register_widget(w3);

        // Start at widget1
        assert_eq!(fm.get_focused_widget(), Some(&w1));

        // Go backward should wrap to last widget
        fm.focus_previous();
        assert_eq!(fm.get_focused_widget(), Some(&w3));

        // Go forward should wrap to first widget
        fm.focus_next();
        assert_eq!(fm.get_focused_widget(), Some(&w1));
    }

    #[test]
    fn test_widget_enabled_state_changes() {
        let mut fm = FocusManager::new();
        let w1 = "widget1";
        let w2 = "widget2";

        fm.register_widget(w1);
        fm.register_widget(w2);

        // Initially widget1 is focused
        assert_eq!(fm.get_focused_widget(), Some(&w1));

        // Disable widget1 after it's focused
        fm.set_widget_enabled(&w1, false);

        // Focus should still be on widget1 (focus doesn't automatically move)
        assert_eq!(fm.get_focused_widget(), Some(&w1));

        // But we can't set focus to it explicitly
        assert!(!fm.set_focus(&w1));

        // Enable it again
        fm.set_widget_enabled(&w1, true);
        assert!(fm.set_focus(&w1));
    }

    #[test]
    fn test_default_implementation() {
        let fm = FocusManager::default();
        assert_eq!(fm.get_focused_widget(), None);
        assert_eq!(fm.get_all_widgets().len(), 0);
    }
}
