use crate::Message;
use crate::component::Draw;
use ratatui::{
    Frame,
    crossterm::event::{Event, KeyCode},
    layout::Rect,
    style::{Color, Style},
    widgets::{Block, Paragraph},
};
use tui_input::Input;
use tui_input::backend::crossterm::EventHandler;
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
enum InputMode {
    Normal,
    #[default]
    Editing,
}
pub struct CInput {
    input: Input,
    /// Current input mode
    input_mode: InputMode,
}

impl Draw for CInput {
    fn render(&mut self, frame: &mut Frame, area: Rect) {
        let focus = self.has_focus();
        // keep 2 for borders and 1 for cursor
        let width = area.width.max(3) - 3;
        let scroll = self.input.visual_scroll(width as usize);
        let style = match (self.input_mode, focus) {
            (InputMode::Normal, true) => Style::default().fg(Color::Cyan),
            (InputMode::Normal, false) => Style::default().fg(Color::DarkGray),
            (InputMode::Editing, true) => Color::Yellow.into(),
            (InputMode::Editing, false) => Style::default().fg(Color::DarkGray),
        };
        let border_style = if focus {
            Style::default().fg(Color::Cyan)
        } else {
            Style::default()
        };

        let title = if focus {
            format!("Input [FOCUSED]")
        } else {
            "Input".to_string()
        };

        let input = Paragraph::new(self.input.value())
            .style(style)
            .scroll((0, scroll as u16))
            .block(Block::bordered().title(title).border_style(border_style));
        frame.render_widget(input, area);

        if self.input_mode == InputMode::Editing && focus {
            // Ratatui hides the cursor unless it's explicitly set. Position the  cursor past the
            // end of the input text and one line down from the border to the input line
            let x = self.input.visual_cursor().max(scroll) - scroll + 1;
            frame.set_cursor_position((area.x + x as u16, area.y + 1))
        }
    }
}

impl crate::component::EventHandler for CInput {
    fn set_focus(&mut self, focused: bool) {
        if !focused {
            self.stop_editing();
        } else {
            self.start_editing();
        }
    }

    fn handle_actions(&mut self, _event: &Message) -> Option<Message> {
        None
    }

    fn has_focus(&self) -> bool {
        self.input_mode == InputMode::Editing
    }
    fn handle_events(&mut self, event: &Event) -> Option<Message> {
        let mut message = None;
        if let Some(key_event) = event.as_key_event() {
            message = match self.input_mode {
                InputMode::Normal => match key_event.code {
                    KeyCode::Enter => {
                        self.start_editing();
                        None
                    }
                    _ => None,
                },
                InputMode::Editing => match key_event.code {
                    KeyCode::Enter => {
                        let m = self.get_current_input();
                        self.reset_value();
                        if !m.is_empty() {
                            Some(Message::SendMessage(m))
                        } else {
                            None
                        }
                    }
                    KeyCode::Esc => {
                        self.stop_editing();
                        None
                    }
                    _ => {
                        self.input.handle_event(&event);
                        None
                    }
                },
            }
        }
        message
    }
}

impl CInput {
    pub fn new() -> Self {
        Self {
            input: Input::new(String::from("")),
            input_mode: InputMode::Editing,
        }
    }
    fn has_focus(&self) -> bool {
        self.input_mode == InputMode::Editing
    }

    fn start_editing(&mut self) {
        self.input_mode = InputMode::Editing
    }

    fn stop_editing(&mut self) {
        self.input_mode = InputMode::Normal
    }

    pub fn get_current_input(&self) -> String {
        self.input.to_string()
    }

    pub fn reset_value(&mut self) {
        self.input.reset();
    }
}
