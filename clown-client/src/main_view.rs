use std::fs::File;
use std::io::Write;

use crate::Message;
use crate::component::Child;
use crate::component::Component;
use crate::event_handler::Event;
use crate::focus_manager::FocusManager;
use crate::input_widget;
use crate::input_widget::CInput;
use crate::model::Model;
use crate::text_widget;
use crate::widget_view;
use clown_core::client::Client;
use clown_core::command::Command;
use clown_core::response::Response;
use clown_core::response::ResponseNumber;
use crossterm::event::KeyCode;
use crossterm::event::KeyEventKind;
use crossterm::event::KeyModifiers;
use ratatui::{
    Frame,
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

fn connect_irc(model: &mut Model) {
    if let Some(connection_config) = model.connection_config.clone() {
        if let Some(irc_config) = model.irc_config.clone() {
            let mut client = Client::new(irc_config, File::create("log.txt").ok());
            let reciever = client.message_receiver();
            let command_sender = client.command_sender();

            model.command_sender = Some(command_sender);
            model.message_reciever = reciever;

            client.spawn(connection_config);
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
        let mut message = None;

        message = match event {
            Event::Crossterm(crossterm::event::Event::Key(key_event)) => {
                if key_event.kind == KeyEventKind::Press && key_event.code == KeyCode::Tab {
                    if key_event.modifiers.contains(KeyModifiers::SHIFT) {
                        self.focus_manager.focus_previous();
                    } else {
                        self.focus_manager.focus_next();
                    }
                    self.update_widget_focus();
                    None
                } else {
                    let mut new_message = None;
                    // Pass event to focused widget
                    for child in self.children().iter_mut() {
                        if child.has_focus() {
                            new_message = child.handle_events(event);
                            break;
                        }
                    }
                    new_message
                }
            }
            Event::Tick => Some(Message::PullIRC),
            _ => None,
        };

        Ok(message)
    }

    fn update(&mut self, model: &mut Model, msg: Message) -> Option<Message> {
        match msg {
            Message::SendMessage(content) => {
                if let Some(parsed_message) = command::parse_command(&content) {
                    match parsed_message {
                        command::ClientCommand::Connect => Some(Message::Connect),
                        command::ClientCommand::Join => Some(Message::Connect),
                        command::ClientCommand::Quit => Some(Message::Quit),
                    }
                } else if let Some(irc_config) = &model.irc_config {
                    model.send_command(clown_core::command::Command::PrivMsg(
                        irc_config.channel.to_string(),
                        content.clone(),
                    ));
                    return self
                        .messages_display
                        .handle_actions(&Message::AddMessageView(content));
                } else {
                    None
                }
            }
            Message::Quit => {
                model.send_command(clown_core::command::Command::Quit(None));
                None
            }
            Message::Connect => {
                connect_irc(model);
                None
            }
            Message::AddMessageView(content) => self
                .messages_display
                .handle_actions(&Message::AddMessageView(content)),
            Message::PullIRC => {
                if let Some(reciever) = model.message_reciever.as_mut()
                    && let Ok(recieved) = reciever.inner.try_recv()
                    && let Some(reply) = recieved.get_reply()
                {
                    match reply {
                        Response::Cmd(command) => match command {
                            Command::PrivMsg(_target, content) => {
                                Some(Message::AddMessageView(content))
                            }
                            _ => None,
                        },
                        Response::Rpl(reply) => match reply {
                            ResponseNumber::Welcome(content) => {
                                if let Some(irc_config) = &model.irc_config {
                                    model.send_command(clown_core::command::Command::Join(
                                        irc_config.channel.to_string(),
                                    ));
                                }
                                Some(Message::AddMessageView(content))
                            }
                            _ => None,
                        },
                    }
                } else {
                    None
                }
            }
            _ => None,
        }
    }
}
