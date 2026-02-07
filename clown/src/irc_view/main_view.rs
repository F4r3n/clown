use self::command::connect_irc;
use self::command::help;
use crate::component::Child;
use crate::component::Component;
use crate::event_handler::Event;
use crate::irc_view::command;
use crate::irc_view::command::ClientCommand;
use crate::irc_view::discuss::discuss_widget;
use crate::irc_view::input::input_widget;
use crate::irc_view::input::input_widget::CInput;
use crate::irc_view::irc_model::IrcModel;
use crate::irc_view::tooltip_widget;
use crate::irc_view::topic_widget;
use crate::irc_view::users_widget;
use crate::message_event::MessageEvent;
use crate::message_irc::message_content::MessageContent;
use crate::message_queue::MessageQueue;
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
use strum::{EnumMessage, IntoEnumIterator};

use tracing::debug;
use tracing::error;
#[derive(Debug, thiserror::Error)]
pub enum MessageError {
    #[error("The message should have a source")]
    MissingSource,
}

const LOG_FLUSH_CHECK_TIMER: u64 = 10;

pub struct MainView<'a> {
    input: Component<'a, CInput>,
    messages_display: Component<'a, discuss_widget::DiscussWidget>,
    list_users_view: Component<'a, users_widget::UsersWidget>,
    topic_view: Component<'a, topic_widget::TopicWidget>,
    tooltip_widget: Component<'a, tooltip_widget::ToolTipDiscussWidget>,

    need_redraw: bool,
    has_focus: bool,

    log_instant: std::time::Instant,
}

impl MainView<'_> {
    pub fn new(current_channel: &str, current_nickname: &str) -> Self {
        let mut cinput = input_widget::CInput::default();
        cinput.add_completion_command_list(
            ClientCommand::iter().map(|v| v.get_message().unwrap_or("")),
        );

        let input = Component::new("input", cinput);
        let list_users_view: Component<'_, users_widget::UsersWidget> =
            Component::new("users_view", users_widget::UsersWidget::new());
        let topic_view: Component<'_, topic_widget::TopicWidget> =
            Component::new("topic_view", topic_widget::TopicWidget::new());
        //list_components.push()
        let messages_display = Component::new(
            "messages",
            discuss_widget::DiscussWidget::new(current_channel, current_nickname.to_string()),
        );
        let tooltip_widget = Component::new("tooltip", tooltip_widget::ToolTipDiscussWidget::new());
        Self {
            list_users_view,
            topic_view,
            input,
            messages_display,
            tooltip_widget,
            need_redraw: false,
            has_focus: true,
            log_instant: std::time::Instant::now(),
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

    fn update_input(&mut self, model: &mut Model, content: &str) -> Option<MessageEvent> {
        if let Some(parsed_message) = command::parse_command(content) {
            match parsed_message {
                command::ClientCommand::Connect => Some(MessageEvent::Connect),
                command::ClientCommand::Quit(message) => {
                    model.send_command(Command::Quit(message));
                    model.running_state = RunningState::Done;
                    None
                }
                command::ClientCommand::Help => Some(help()),
                command::ClientCommand::Nick(new_nick) => {
                    let _ = model.set_nickname(new_nick.clone());
                    model.send_command(Command::Nick(new_nick.clone()));
                    None
                }
                command::ClientCommand::Topic(topic) => {
                    model
                        .send_command(Command::Topic(model.current_channel.clone(), topic.clone()));
                    None
                }
                command::ClientCommand::Spell(language) => {
                    Some(MessageEvent::SpellChecker(language))
                }
                command::ClientCommand::Join(channel) => {
                    model.send_command(Command::Join(channel.clone())); //the server will check
                    None
                }
                command::ClientCommand::Part(channel, reason) => {
                    let chanel = channel.unwrap_or_else(|| model.current_channel.clone());
                    model.send_command(Command::Part(chanel.clone(), reason.clone())); //the server will check
                    None
                }
                command::ClientCommand::Action(content) => {
                    let nickname = model.get_nickname().to_string();

                    model.send_command(clown_core::command::Command::PrivMsg(
                        model.current_channel.to_string(),
                        format!("\x01ACTION {}\x01", content.clone()),
                    ));
                    Some(MessageEvent::ActionMsg(
                        nickname,
                        model.current_channel.to_string(),
                        content,
                    ))
                }
                command::ClientCommand::PrivMSG(channel, content) => {
                    model.send_command(clown_core::command::Command::PrivMsg(
                        channel.clone(),
                        content.clone(),
                    ));

                    let nickname = model.get_nickname().to_string();
                    Some(MessageEvent::PrivMsg(nickname, channel, content))
                }
                command::ClientCommand::Unknown(command_name) => {
                    self.messages_display.handle_actions(
                        &model.irc_model,
                        &MessageEvent::AddMessageView(
                            None,
                            MessageContent::new_error(format!(
                                "Unknown command {}",
                                command_name.unwrap_or_default()
                            )),
                        ),
                    )
                }
            }
        } else {
            let nickname = model.get_nickname().to_string();
            model.send_command(clown_core::command::Command::PrivMsg(
                model.current_channel.to_string(),
                content.to_string(),
            ));

            Some(MessageEvent::PrivMsg(
                nickname,
                model.current_channel.to_string(),
                content.to_string(),
            ))
        }
    }

    fn handle_irc(&mut self, model: &mut Model, messages: &mut MessageQueue) {
        let mut received_error = false;
        let message = if model.running_state == RunningState::Start {
            model.running_state = RunningState::Running;

            if model.is_autojoin() {
                Some(MessageEvent::Connect)
            } else {
                None
            }
        } else if let Some(msg) = model.pull_server_error() {
            received_error = true;
            //Received an error
            Some(MessageEvent::AddMessageView(
                None,
                MessageContent::new_error(msg),
            ))
        } else {
            Some(MessageEvent::PullIRC)
        };
        if let Some(message) = message {
            messages.push_message(message);
        }
        if received_error {
            //Try to reconnect
            if model.is_irc_finished() {
                model.irc_connection = None;
            }
            messages
                .push_message_with_time(MessageEvent::Connect, std::time::Duration::from_secs(2));
        }
    }

    fn handle_tick(&mut self, model: &mut Model, event: &Event, messages: &mut MessageQueue) {
        self.handle_irc(model, messages);
        if self.log_instant.elapsed() > std::time::Duration::from_secs(LOG_FLUSH_CHECK_TIMER) {
            if let Err(e) = model.flush_log() {
                tracing::error!(error = %e, "Log flush failed");
            }
            self.log_instant = std::time::Instant::now();
        }
        for mut child in self.children() {
            if let Some(message) = child.handle_events(event) {
                messages.push_message(message);
            }
        }
    }

    fn update_pull_irc(&mut self, model: &mut Model, messages: &mut MessageQueue) {
        while let Some(recieved) = model.pull_server_message() {
            let reply = recieved.reply();
            let source = recieved.source().map(|v| v.to_string());

            debug!("{:?}", recieved);
            //log_info_sync(format!("{reply:?}\n").as_str());
            match reply {
                Response::Cmd(command) => match command {
                    Command::PrivMsg(target, content) => {
                        if let Some(source) = source {
                            let target = if target.eq_ignore_ascii_case(model.get_nickname()) {
                                source.clone()
                            } else {
                                target
                            };

                            if content.starts_with("\x01ACTION") {
                                if let Some(parsed_content) = content.get(8..content.len() - 1) {
                                    messages.push_message(MessageEvent::ActionMsg(
                                        source,
                                        target,
                                        parsed_content.to_string(),
                                    ));
                                }
                            } else {
                                messages
                                    .push_message(MessageEvent::PrivMsg(source, target, content));
                            }
                        } else {
                            tracing::error!(error = %MessageError::MissingSource, "PrivMSG");
                        }
                    }
                    Command::Nick(new_user) => {
                        if let Some(source) = source {
                            messages.push_message(MessageEvent::ReplaceUser(source, new_user));
                        } else {
                            tracing::error!(error = %MessageError::MissingSource, "Nick");
                        }
                    }
                    Command::Notice(target, message) => {
                        //Display a notice directly to the user current channel
                        messages.push_message(MessageEvent::AddMessageView(
                            Some(target),
                            MessageContent::new_notice(source, message),
                        ));
                    }
                    Command::Topic(channel, topic) => {
                        messages.push_message(MessageEvent::SetTopic(source, channel, topic));
                    }
                    Command::Quit(reason) => {
                        if let Some(source) = source {
                            messages.push_message(MessageEvent::Quit(source, reason));
                        } else {
                            tracing::error!(error = %MessageError::MissingSource, "Quit");
                        }
                    }
                    Command::Part(channel, _reason) => {
                        if let Some(source) = source {
                            messages.push_message(MessageEvent::Part(channel.to_string(), source));
                        } else {
                            tracing::error!(error = %MessageError::MissingSource, "Part");
                        }
                    }
                    Command::Join(channel) => {
                        if let Some(source) = source {
                            //Create a new 'user' as IRC-Server
                            messages
                                .push_message(MessageEvent::Join(channel.clone(), source.clone()));
                        } else {
                            tracing::error!(error = %MessageError::MissingSource, "Join");
                        }
                    }
                    Command::Error(_err) => messages.push_message(MessageEvent::DisConnect),
                    _ => {}
                },
                Response::Rpl(reply) => match reply {
                    ResponseNumber::Welcome(content) => {
                        model.reset_retry();

                        model.send_command(clown_core::command::Command::Join(
                            model.get_login_channel().to_string(),
                        ));
                        //TODO: pass welcome message directly to the components
                        //Create a new 'user' as IRC-Server
                        if let Some(source) = source.clone() {
                            messages.push_message(MessageEvent::JoinServer(source));
                        } else {
                            tracing::error!(error = %MessageError::MissingSource, "Welcome");
                        }

                        messages.push_message(MessageEvent::AddMessageView(
                            source.clone(),
                            MessageContent::new(source, content),
                        ));
                    }
                    ResponseNumber::YourHost(content)
                    | ResponseNumber::Created(content)
                    | ResponseNumber::MyInfo(content)
                    | ResponseNumber::Bounce(content)
                    | ResponseNumber::LUserClient(content)
                    | ResponseNumber::LUserOp(content)
                    | ResponseNumber::LUserUnknown(content)
                    | ResponseNumber::LUserChannels(content)
                    | ResponseNumber::LUserMe(content)
                    | ResponseNumber::MOTD(content)
                    | ResponseNumber::MOTDStart2(content)
                    | ResponseNumber::MOTDStart(content)
                    | ResponseNumber::EndOfMOTD(content) => {
                        messages.push_message(MessageEvent::AddMessageView(
                            source.clone(),
                            MessageContent::new(source, content),
                        ));
                    }
                    ResponseNumber::NameReply(_symbol, channel, list_users) => {
                        //info!("{} {} {:?}", symbol, channel, list_users);
                        messages.push_message(MessageEvent::UpdateUsers(channel, list_users));
                    }
                    ResponseNumber::Topic(channel, topic) => {
                        messages.push_message(MessageEvent::SetTopic(None, channel, topic));
                    }
                    ResponseNumber::Err(_, content) => {
                        messages.push_message(MessageEvent::AddMessageView(
                            None,
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

impl widget_view::WidgetView for MainView<'_> {
    fn need_redraw(&mut self, _model: &mut Model) -> bool {
        if self.need_redraw {
            return self.need_redraw;
        }

        for child in self.children().iter_mut() {
            if child.need_redraw() {
                return true;
            }
        }
        false
    }
    fn view(&mut self, model: &mut Model, frame: &mut Frame<'_>) {
        if self.need_redraw {
            self.need_redraw = false;
        }
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
                self.messages_display
                    .render(&model.irc_model, frame, *message_area);
                self.tooltip_widget
                    .render(&model.irc_model, frame, *message_area);
            }

            if let Some(list_users) = top_layout.get(1) {
                self.list_users_view
                    .render(&model.irc_model, frame, *list_users);
            }
        }

        // Render widgets
        if let Some(input_area) = main_layout.get(2) {
            self.input.render(&model.irc_model, frame, *input_area);
        }

        if let Some(topic_area) = main_layout.first() {
            self.topic_view.render(&model.irc_model, frame, *topic_area);
        }
    }

    fn handle_event(&mut self, model: &mut Model, event: &Event, messages: &mut MessageQueue) {
        // Handle focus switching first
        match event {
            Event::Crossterm(crossterm::event::Event::Key(_)) => {
                // Pass event to focused widget
                if self.has_focus {
                    for child in self.children().iter_mut() {
                        if let Some(new_message) = child.handle_events(event) {
                            messages.push_message(new_message);
                        }
                    }
                }
            }
            Event::Crossterm(crossterm::event::Event::Resize(_, _)) => {
                self.need_redraw = true;
            }
            Event::Crossterm(crossterm::event::Event::FocusGained) => {
                self.has_focus = true;
            }
            Event::Crossterm(crossterm::event::Event::FocusLost) => {
                self.has_focus = false;
            }
            Event::Crossterm(crossterm::event::Event::Paste(_)) => {
                self.input.handle_events(event);
            }
            Event::Crossterm(crossterm::event::Event::Mouse(mouse_event)) => {
                if let Some(id) = self.get_id_from_row_col(mouse_event.column, mouse_event.row) {
                    for child in self.children().iter_mut() {
                        if child.get_id().eq(&id)
                            && let Some(new_message) = child.handle_events(event)
                        {
                            messages.push_message(new_message);
                        }
                    }
                }
            }
            Event::Tick => {
                self.handle_tick(model, event, messages);
            }
            _ => {}
        };
    }

    fn update(&mut self, model: &mut Model, msg: MessageEvent, messages: &mut MessageQueue) {
        match msg {
            MessageEvent::MessageInput(content) => {
                //crossterm sends with only \r, lines() wont work
                for m in content.split(['\r', '\n']) {
                    if m.is_empty() {
                        continue;
                    }
                    if let Some(v) = self.update_input(model, m) {
                        messages.push_message(v)
                    }
                }
            }
            #[allow(clippy::print_stdout)]
            MessageEvent::Bel => {
                println!("{}", 0x07 as char);
            }
            MessageEvent::OpenWeb(url) => {
                if let Err(e) = open::that(&url) {
                    error!("Try to open {}, {}", url.clone(), e);
                }
            }
            MessageEvent::Connect => {
                messages.push_message(MessageEvent::AddMessageView(
                    None,
                    MessageContent::new_info(format!(
                        "Try to connect to {}...",
                        model.get_address().unwrap_or("No address")
                    )),
                ));
                if let Some(v) = connect_irc(model) {
                    messages.push_message(v)
                }
            }
            MessageEvent::SelectChannel(ref channel) => {
                model.current_channel = channel.to_string();
                for mut child in self.children() {
                    child.handle_actions(&model.irc_model, &msg);
                }
            }
            MessageEvent::DisConnect => {
                if !model.is_irc_finished() {
                    model.send_command(clown_core::command::Command::Quit(None));
                } else {
                    messages.push_message(MessageEvent::AddMessageView(
                        None,
                        MessageContent::new(None, "Disconnected".to_string()),
                    ));
                }
            }
            MessageEvent::PullIRC => self.update_pull_irc(model, messages),
            _ => {
                if let Err(e) = model.log(&msg) {
                    tracing::error!(error = %e, "Cannot write logs");
                }
                for mut child in self.children() {
                    if let Some(msg) = child.handle_actions(&model.irc_model, &msg) {
                        messages.push_message(msg);
                    }
                }
                model.irc_model.handle_action(&msg);
            }
        };
    }
}
