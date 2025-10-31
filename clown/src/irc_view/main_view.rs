use std::collections::VecDeque;

use crate::MessageEvent;
use crate::command::connect_irc;
use crate::command::help;
use crate::component::Child;
use crate::component::Component;
use crate::event_handler::Event;
use crate::irc_view::discuss_widget;
use crate::irc_view::input_widget;
use crate::irc_view::input_widget::CInput;
use crate::irc_view::message_content::MessageContent;
use crate::irc_view::tooltip_widget;
use crate::irc_view::topic_widget;
use crate::irc_view::users_widget;
use crate::model::Model;
use crate::model::RunningState;
use crate::widget_view;
use clown_core::command::Command;
use clown_core::response::Response;
use clown_core::response::ResponseNumber;
use ratatui::layout::Position;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout},
};

pub struct MainView<'a> {
    input: Component<'a, CInput>,
    messages_display: Component<'a, discuss_widget::DiscussWidget>,
    list_users_view: Component<'a, users_widget::UsersWidget>,
    topic_view: Component<'a, topic_widget::TopicWidget>,
    tooltip_widget: Component<'a, tooltip_widget::ToolTipDiscussWidget>,
}
impl MainView<'_> {
    pub fn new(current_channel: &str) -> Self {
        let input = Component::new("input", input_widget::CInput::new());
        let list_users_view: Component<'_, users_widget::UsersWidget> = Component::new(
            "users_view",
            users_widget::UsersWidget::new(current_channel),
        );
        let topic_view: Component<'_, topic_widget::TopicWidget> =
            Component::new("topic_view", topic_widget::TopicWidget::new());
        //list_components.push()
        let messages_display = Component::new(
            "messages",
            discuss_widget::DiscussWidget::new(current_channel),
        );
        let tooltip_widget = Component::new("tooltip", tooltip_widget::ToolTipDiscussWidget::new());
        Self {
            list_users_view,
            topic_view,
            input,
            messages_display,
            tooltip_widget,
        }
    }

    fn get_id_from_row_col(&mut self, column: u16, row: u16) -> Option<String> {
        let mut id: Option<String> = None;

        for child in self.children().iter() {
            if child.get_area().contains(Position::new(column, row)) {
                id = Some(child.get_id().to_string());
                break;
            }
        }
        id
    }

    fn children(&mut self) -> [Component<'_, Child<'_>>; 5] {
        [
            self.input.to_child_mut(),
            self.messages_display.to_child_mut(),
            self.list_users_view.to_child_mut(),
            self.topic_view.to_child_mut(),
            self.tooltip_widget.to_child_mut(),
        ]
    }

    fn update_input(&mut self, model: &mut Model, content: String) -> Option<MessageEvent> {
        if let Some(parsed_message) = command::parse_command(&content) {
            match parsed_message {
                command::ClientCommand::Connect => Some(MessageEvent::Connect),
                command::ClientCommand::Quit(message) => {
                    Some(MessageEvent::Quit(if message.is_empty() {
                        None
                    } else {
                        Some(message)
                    }))
                }
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
                    MessageContent::new(Some(nickname), content.as_str()),
                ))
        }
    }

    fn update_pull_irc(&mut self, model: &mut Model, messages: &mut VecDeque<MessageEvent>) {
        if let Some(recieved) = model.pull_server_message() {
            let reply = recieved.reply();
            let source = recieved.source().map(|v| v.to_string());
            //log_info_sync(format!("{reply:?}\n").as_str());
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
                            MessageContent::new_message(
                                source,
                                content.as_str(),
                                &model.config.login_config.nickname,
                            ),
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
                            MessageContent::new_info(
                                format!("{} has quit", source.clone()).as_str(),
                            ),
                        ));
                    }
                    Command::Join(_) => {
                        let source = source.unwrap_or_default();

                        messages.push_back(MessageEvent::JoinUser(source.clone()));
                        if !source.eq(&model.config.login_config.nickname) {
                            messages.push_back(MessageEvent::AddMessageView(
                                model.current_channel.to_string(),
                                MessageContent::new_info(
                                    format!("{} has joined", source.clone()).as_str(),
                                ),
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
                            MessageContent::new(source, content.as_str()),
                        ));
                    }
                    ResponseNumber::NameReply(list_users) => {
                        messages.push_back(MessageEvent::UpdateUsers(list_users));
                    }
                    ResponseNumber::Topic(topic) => {
                        messages.push_back(MessageEvent::SetTopic(topic));
                    }
                    ResponseNumber::Err(_, content) => {
                        messages.push_back(MessageEvent::AddMessageView(
                            model.current_channel.to_string(),
                            MessageContent::new_error(content),
                        ));
                    }
                    _ => {}
                },
                Response::Unknown(_) => {}
            };
        }
    }
}

use crate::command;
impl widget_view::WidgetView for MainView<'_> {
    fn view(&mut self, _model: &mut Model, frame: &mut Frame<'_>) {
        // Create layout
        let main_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),       // Topic area
                Constraint::Percentage(100), // Messages area
                Constraint::Length(2),       // Input area
            ])
            .split(frame.area());

        if let Some(message_area_layout) = main_layout.get(1) {
            let top_layout = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Percentage(100), // Messages
                    Constraint::Min(15),         // List Users
                ])
                .split(*message_area_layout);

            if let Some(message_area) = top_layout.first() {
                self.messages_display.render(frame, *message_area);
                self.tooltip_widget.render(frame, *message_area);
            }

            if let Some(list_users) = top_layout.get(1) {
                self.list_users_view.render(frame, *list_users);
            }
        }

        // Render widgets
        if let Some(input_area) = main_layout.get(2) {
            self.input.render(frame, *input_area);
        }

        if let Some(topic_area) = main_layout.first() {
            self.topic_view.render(frame, *topic_area);
        }
    }

    fn handle_event(
        &mut self,
        model: &mut Model,
        event: &Event,
        messages: &mut VecDeque<MessageEvent>,
    ) {
        // Handle focus switching first

        match event {
            Event::Crossterm(crossterm::event::Event::Key(_)) => {
                // Pass event to focused widget
                for child in self.children().iter_mut() {
                    if let Some(new_message) = child.handle_events(event) {
                        messages.push_back(new_message);
                    }
                }
            }
            Event::Crossterm(crossterm::event::Event::Mouse(mouse_event)) => {
                if let Some(id) = self.get_id_from_row_col(mouse_event.column, mouse_event.row) {
                    for child in self.children().iter_mut() {
                        if child.get_id().eq(&id) {
                            if let Some(new_message) = child.handle_events(event) {
                                messages.push_back(new_message);
                            }
                        }
                    }
                }
            }
            Event::Tick => {
                let mut received_error = false;
                let message = if model.running_state == RunningState::Start {
                    model.running_state = RunningState::Running;

                    if model.config.client_config.auto_join {
                        Some(MessageEvent::Connect)
                    } else {
                        None
                    }
                } else if let Some(msg) = model.pull_server_error() {
                    received_error = true;
                    //Received an error
                    Some(MessageEvent::AddMessageView(
                        model.current_channel.clone(),
                        MessageContent::new_error(msg),
                    ))
                } else {
                    Some(MessageEvent::PullIRC)
                };

                if let Some(message) = message {
                    messages.push_back(message);
                }

                if received_error {
                    //Try to reconnect
                    if model.is_irc_finished() {
                        model.irc_connection = None;
                    }
                    messages.push_back(MessageEvent::Connect);
                }
            }
            _ => {}
        };
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
            MessageEvent::Quit(message) => {
                model.send_command(clown_core::command::Command::Quit(message));
            }
            MessageEvent::Connect => {
                messages.push_back(MessageEvent::AddMessageView(
                    model.current_channel.to_string(),
                    MessageContent::new_info(
                        format!(
                            "Try to connect to {}...",
                            model.config.connection_config.address
                        )
                        .as_str(),
                    ),
                ));
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
                        MessageContent::new(None, "Disconnected"),
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
