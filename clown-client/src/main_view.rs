use crate::Message;
use crate::component::Child;
use crate::component::Component;
use crate::focus_manager::FocusManager;
use crate::input_widget;
use crate::input_widget::CInput;
use crate::model::Model;
use crate::text_widget;
use crate::widget_view;
use ratatui::{
    Frame,
    crossterm::event::{Event, KeyCode, KeyEventKind},
    layout::{Constraint, Direction, Layout},
};

pub struct MainView<'a> {
    input: Component<'a, CInput>,
    messages_display: Component<'a, text_widget::TextWidget>,
    focus_manager: FocusManager<'a>,
}
impl<'a> MainView<'a> {
    pub fn new() -> Self {
        let mut focus_manager = FocusManager::new();
        let mut input = Component::new("input", input_widget::CInput::new());
        //list_components.push()
        let messages_display = Component::new("messages", text_widget::TextWidget::new(vec![]));

        // Register widgets with focus manager
        focus_manager.register_widget(input.get_id());
        focus_manager.register_widget(messages_display.get_id());

        // Set initial focus to input
        input.set_focus(true);

        Self {
            input,
            messages_display,
            focus_manager,
        }
    }

    fn children(&mut self) -> Vec<Component<Child<'_>>> {
        vec![
            self.input.to_child_mut(),
            self.messages_display.to_child_mut(),
        ]
    }

    fn update_widget_focus(&mut self) {
        let focused_widget = self.focus_manager.get_focused_widget().cloned();
        if let Some(focused_widget) = focused_widget {
            for child in self.children().iter_mut() {
                child.set_focus(*child.get_id() == focused_widget);
            }
        }
    }
}
use crate::command;
impl<'a> widget_view::WidgetView for MainView<'a> {
    fn view(&mut self, _model: &mut Model, frame: &mut Frame) {
        // Create layout
        let main_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(10),   // Messages area
                Constraint::Length(4), // Input area
            ])
            .split(frame.area());

        let top_layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(70), // Messages
            ])
            .split(main_layout[0]);

        // Render widgets
        self.messages_display.render(frame, top_layout[0]);
        self.input.render(frame, main_layout[1]);
    }

    fn handle_event(
        &mut self,
        _model: &mut Model,
        event: &Event,
    ) -> color_eyre::Result<Option<Message>> {
        // Handle focus switching first
        if let Event::Key(key) = event {
            if key.kind == KeyEventKind::Press {
                match key.code {
                    KeyCode::Tab => {
                        if key
                            .modifiers
                            .contains(ratatui::crossterm::event::KeyModifiers::SHIFT)
                        {
                            self.focus_manager.focus_previous();
                        } else {
                            self.focus_manager.focus_next();
                        }
                        self.update_widget_focus();
                        return Ok(None);
                    }
                    _ => {}
                }
            }
        }

        // Pass event to focused widget
        let mut message = None;
        for child in self.children().iter_mut() {
            if child.has_focus() {
                message = child.handle_events(event);
                break;
            }
        }

        Ok(message)
    }

    fn update(&mut self, model: &mut Model, msg: Message) -> Option<Message> {
        match msg {
            Message::SendMessage(content) => {
                if let Some(parsed_message) = command::parse_command(&content) {
                    match parsed_message {
                        command::Command::Connect => Some(Message::Connect),
                        command::Command::Quit => Some(Message::Quit),
                        _ => None,
                    }
                } else {
                    if let Some(command_sender) = model.command_sender.as_mut() {
                        if let Some(irc_config) = &model.irc_config {
                            let _ = command_sender.send(clown_core::command::Command::PrivMsg(
                                irc_config.channel.to_string(),
                                content.clone(),
                            ));
                        }
                    }
                    self.messages_display
                        .handle_actions(&Message::AddMessage(content))
                }
            }
            Message::Quit => {
                if let Some(command_sender) = model.command_sender.as_mut() {
                    let _ = command_sender.send(clown_core::command::Command::Quit(None));
                }
                None
            }
            _ => None,
        }
    }
}
