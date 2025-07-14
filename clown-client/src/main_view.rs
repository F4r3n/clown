use std::fs::File;

use crate::MessageEvent;
use crate::component::Child;
use crate::component::Component;
use crate::event_handler::Event;
use crate::focus_manager::FocusManager;
use crate::input_widget;
use crate::input_widget::CInput;
use crate::model::Model;
use crate::text_widget;
use crate::text_widget::MessageContent;
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

    fn update_input(&mut self, model: &mut Model, content: String) -> Option<MessageEvent> {
        if let Some(parsed_message) = command::parse_command(&content) {
            match parsed_message {
                command::ClientCommand::Connect => Some(MessageEvent::Connect),
                command::ClientCommand::Join => Some(MessageEvent::Join),
                command::ClientCommand::Quit => Some(MessageEvent::Quit),
            }
        } else if let Some(irc_config) = &model.irc_config {
            let nickname = irc_config.nickname.clone();
            model.send_command(clown_core::command::Command::PrivMsg(
                irc_config.channel.to_string(),
                content.clone(),
            ));
            return self
                .messages_display
                .handle_actions(&MessageEvent::AddMessageView(MessageContent::new(
                    Some(nickname),
                    content,
                )));
        } else {
            None
        }
    }

    fn update_pull_irc(&mut self, model: &mut Model) -> Option<MessageEvent> {
        if let Some(reciever) = model.message_reciever.as_mut()
            && let Ok(recieved) = reciever.inner.try_recv()
            && let Some(reply) = recieved.get_reply()
        {
            let source = recieved.get_source().map(|v| v.to_string());
            match reply {
                Response::Cmd(command) => match command {
                    Command::PrivMsg(_target, content) => Some(MessageEvent::AddMessageView(
                        MessageContent::new(source, content),
                    )),
                    _ => None,
                },
                Response::Rpl(reply) => match reply {
                    ResponseNumber::Welcome(content) => {
                        if let Some(irc_config) = &model.irc_config {
                            model.send_command(clown_core::command::Command::Join(
                                irc_config.channel.to_string(),
                            ));
                        }
                        Some(MessageEvent::AddMessageView(MessageContent::new(
                            source, content,
                        )))
                    }
                    _ => None,
                },
            }
        } else {
            None
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
    ) -> color_eyre::Result<Option<MessageEvent>> {
        // Handle focus switching first

        let message = match event {
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
            Event::Tick => Some(MessageEvent::PullIRC),
            _ => None,
        };

        Ok(message)
    }

    fn update(&mut self, model: &mut Model, msg: MessageEvent) -> Option<MessageEvent> {
        match msg {
            MessageEvent::MessageInput(content) => self.update_input(model, content),
            MessageEvent::Quit => {
                model.send_command(clown_core::command::Command::Quit(None));
                None
            }
            MessageEvent::Connect => {
                connect_irc(model);
                None
            }
            MessageEvent::AddMessageView(content) => self
                .messages_display
                .handle_actions(&MessageEvent::AddMessageView(content)),
            MessageEvent::PullIRC => self.update_pull_irc(model),
            _ => None,
        }
    }
}
