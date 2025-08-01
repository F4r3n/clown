use std::collections::VecDeque;

use crate::MessageEvent;
use crate::command::connect_irc;
use crate::command::help;
use crate::component::Child;
use crate::component::Component;
use crate::event_handler::Event;
use crate::focus_manager::FocusManager;
use crate::irc_view::input_widget;
use crate::irc_view::input_widget::CInput;
use crate::irc_view::message_content::MessageContent;
use crate::irc_view::text_widget;
use crate::irc_view::topic_widget;
use crate::irc_view::users_widget;
use crate::model::Model;
use crate::model::RunningState;
use crate::widget_view;
use clown_core::command::Command;
use clown_core::response::Response;
use clown_core::response::ResponseNumber;
use crossterm::event::KeyCode;
use crossterm::event::KeyEventKind;
use crossterm::event::KeyModifiers;
use ratatui::layout::Position;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout},
};

pub struct MainView<'a> {
    input: Component<'a, CInput>,
    messages_display: Component<'a, text_widget::TextWidget>,
    list_users_view: Component<'a, users_widget::UsersWidget>,
    topic_view: Component<'a, topic_widget::TopicWidget>,

    focus_manager: FocusManager<'a>,
}
impl<'a> MainView<'a> {
    pub fn new(current_channel: &str) -> Self {
        let mut focus_manager = FocusManager::new();
        let mut input = Component::new("input", input_widget::CInput::new());
        let list_users_view: Component<'_, users_widget::UsersWidget> = Component::new(
            "users_view",
            users_widget::UsersWidget::new(current_channel),
        );
        let topic_view: Component<'_, topic_widget::TopicWidget> =
            Component::new("topic_view", topic_widget::TopicWidget::new());
        //list_components.push()
        let messages_display =
            Component::new("messages", text_widget::TextWidget::new(current_channel));

        // Register widgets with focus manager
        focus_manager.register_widget(input.get_id());
        focus_manager.register_widget(messages_display.get_id());
        focus_manager.register_widget(list_users_view.get_id());
        //No topic focus

        // Set initial focus to input
        input.set_focus(true);

        Self {
            list_users_view,
            topic_view,
            input,
            messages_display,
            focus_manager,
        }
    }

    fn get_id_event(&mut self, column: u16, row: u16) -> Option<String> {
        let mut id: Option<String> = None;

        for child in self.children().iter() {
            if child.get_area().contains(Position::new(column, row)) {
                id = Some(child.get_id().to_string());
                break;
            }
        }
        id
    }

    fn children(&mut self) -> Vec<Component<Child<'_>>> {
        vec![
            self.input.to_child_mut(),
            self.messages_display.to_child_mut(),
            self.list_users_view.to_child_mut(),
            self.topic_view.to_child_mut(),
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
                command::ClientCommand::Quit => Some(MessageEvent::Quit),
                command::ClientCommand::Help => Some(help(&model.current_channel)),
                command::ClientCommand::Nick(new_nick) => {
                    model.config.login_config.nickname = new_nick;
                    model.send_command(Command::Nick(model.config.login_config.nickname.clone()));
                    None
                }
            }
        } else {
            let nickname = model.config.login_config.nickname.clone();
            model.send_command(clown_core::command::Command::PrivMsg(
                model.current_channel.to_string(),
                content.clone(),
            ));
            self.messages_display
                .handle_actions(&MessageEvent::AddMessageView(
                    model.current_channel.to_string(),
                    Box::new(MessageContent::new(Some(nickname), content.as_str())),
                ))
        }
    }

    fn update_pull_irc(&mut self, model: &mut Model, messages: &mut VecDeque<MessageEvent>) {
        if let Some(reciever) = model.message_reciever.as_mut()
            && let Ok(recieved) = reciever.inner.try_recv()
            && let Some(reply) = recieved.get_reply()
        {
            let source = recieved.get_source().map(|v| v.to_string());

            match reply {
                Response::Cmd(command) => match command {
                    Command::PrivMsg(target, content) => {
                        let from = if target.eq(&model.config.login_config.nickname) {
                            source.clone().unwrap_or_default()
                        } else {
                            target
                        };
                        if !from.eq(&model.current_channel) {
                            messages.push_back(MessageEvent::HighlightUser(from.clone()));
                        }
                        messages.push_back(MessageEvent::AddMessageView(
                            from,
                            Box::new(MessageContent::new_message(
                                source,
                                content.as_str(),
                                &model.config.login_config.nickname,
                            )),
                        ));
                    }
                    Command::Nick(new_user) => messages.push_back(MessageEvent::ReplaceUser(
                        source.unwrap_or_default(),
                        new_user,
                    )),
                    Command::Topic(_, topic) => messages.push_back(MessageEvent::SetTopic(topic)),
                    Command::Quit(_) => {
                        let source = source.unwrap_or_default();
                        messages.push_back(MessageEvent::RemoveUser(source.clone()));
                        messages.push_back(MessageEvent::AddMessageView(
                            model.current_channel.to_string(),
                            Box::new(MessageContent::new_info(
                                format!("{} has quit", source.clone()).as_str(),
                            )),
                        ));
                    }
                    Command::Join(_) => {
                        let source = source.unwrap_or_default();

                        messages.push_back(MessageEvent::JoinUser(source.clone()));
                        if !source.eq(&model.config.login_config.nickname) {
                            messages.push_back(MessageEvent::AddMessageView(
                                model.current_channel.to_string(),
                                Box::new(MessageContent::new_info(
                                    format!("{} has joined", source.clone()).as_str(),
                                )),
                            ));
                        }
                    }
                    Command::Error(_err) => messages.push_back(MessageEvent::DisConnect),
                    _ => {}
                },
                Response::Rpl(reply) => match reply {
                    ResponseNumber::Welcome(content) => {
                        model.send_command(clown_core::command::Command::Join(
                            model.config.login_config.channel.to_string(),
                        ));

                        messages.push_back(MessageEvent::AddMessageView(
                            model.current_channel.to_string(),
                            Box::new(MessageContent::new(source, content.as_str())),
                        ));
                    }
                    ResponseNumber::NameReply(list_users) => {
                        messages.push_back(MessageEvent::UpdateUsers(list_users));
                    }
                    ResponseNumber::Topic(topic) => {
                        messages.push_back(MessageEvent::SetTopic(topic))
                    }
                    ResponseNumber::Err(_, content) => {
                        messages.push_back(MessageEvent::AddMessageView(
                            model.current_channel.to_string(),
                            Box::new(MessageContent::new_error(content)),
                        ));
                    }
                    _ => {}
                },
            };
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
                Constraint::Length(1),       // Topic area
                Constraint::Percentage(100), // Messages area
                Constraint::Length(4),       // Input area
            ])
            .split(frame.area());

        let top_layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(100), // Messages
                Constraint::Min(15),         // List Users
            ])
            .split(main_layout[1]);

        // Render widgets
        self.messages_display.render(frame, top_layout[0]);
        self.list_users_view.render(frame, top_layout[1]);
        self.input.render(frame, main_layout[2]);
        self.topic_view.render(frame, main_layout[0]);
    }

    fn handle_event(
        &mut self,
        model: &mut Model,
        event: &Event,
    ) -> color_eyre::Result<Option<MessageEvent>> {
        // Handle focus switching first

        let message = match event {
            Event::Crossterm(crossterm::event::Event::Key(key_event)) => {
                if key_event.kind == KeyEventKind::Press
                    && key_event.modifiers.contains(KeyModifiers::CONTROL)
                    && key_event.code == KeyCode::Char('p')
                {
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
            Event::Crossterm(crossterm::event::Event::Mouse(mouse_event)) => {
                if let Some(id) = self.get_id_event(mouse_event.column, mouse_event.row) {
                    if crossterm::event::MouseEventKind::Down(crossterm::event::MouseButton::Left)
                        == mouse_event.kind
                    {
                        self.focus_manager.set_focus(&id);
                        self.update_widget_focus();
                    } else if crossterm::event::MouseEventKind::ScrollDown == mouse_event.kind
                        || crossterm::event::MouseEventKind::ScrollUp == mouse_event.kind
                    {
                        for child in self.children().iter_mut() {
                            if child.get_id().eq(&id) {
                                child.handle_events(event);
                                break;
                            }
                        }
                    } else {
                        for child in self.children().iter_mut() {
                            if child.has_focus() {
                                child.handle_events(event);
                                break;
                            }
                        }
                    }
                }

                None
            }

            Event::Tick => {
                if model.running_state == RunningState::Start {
                    model.running_state = RunningState::Running;

                    if model.config.client_config.auto_join {
                        connect_irc(model)
                    } else {
                        None
                    }
                } else {
                    Some(MessageEvent::PullIRC)
                }
            }
            _ => None,
        };

        Ok(message)
    }

    fn update(
        &mut self,
        model: &mut Model,
        msg: MessageEvent,
        messages: &mut VecDeque<MessageEvent>,
    ) {
        match msg {
            MessageEvent::MessageInput(content) => {
                if let Some(v) = self.update_input(model, content) {
                    messages.push_back(v)
                }
            }
            MessageEvent::Quit => {
                model.send_command(clown_core::command::Command::Quit(None));
            }
            MessageEvent::Connect => {
                if let Some(v) = connect_irc(model) {
                    messages.push_back(v)
                }
            }
            MessageEvent::SelectChannel(ref channel) => {
                model.current_channel = channel.to_string();
                self.messages_display.handle_actions(&msg);
            }
            MessageEvent::DisConnect => {
                if !model.is_irc_finished() {
                    model.send_command(clown_core::command::Command::Quit(None));
                } else {
                    messages.push_back(MessageEvent::AddMessageView(
                        model.current_channel.to_string(),
                        Box::new(MessageContent::new(None, "Disconnected".into())),
                    ));
                }
            }
            MessageEvent::PullIRC => self.update_pull_irc(model, messages),
            _ => {
                for mut child in self.children() {
                    child.handle_actions(&msg);
                }
            }
        };
    }
}
