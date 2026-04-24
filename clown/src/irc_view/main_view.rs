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
use crate::irc_view::session::Session;
use crate::irc_view::tooltip_widget;
use crate::irc_view::topic_widget;
use crate::irc_view::users_widget;
use crate::message_event::MessageEvent;
use crate::message_irc::message_logger;
use crate::message_irc::message_logger::MessageLogger;
use crate::message_queue::MessageQueue;
use crate::model::Model;
use crate::model::RunningState;
use crate::model::ServerID;
use crate::model::StoredConfig;
use crate::widget_view;
use clown_core::command::Command;
use clown_core::conn::ConnectionConfig;
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

    //TODO move them into their own struct
    log_instant: std::time::Instant,
    logger: message_logger::MessageLogger,
}

impl MainView<'_> {
    pub fn new() -> Self {
        let mut cinput = input_widget::CInput::default();
        cinput.add_completion_command_list(
            ClientCommand::iter().map(|v| v.get_message().unwrap_or("").to_string()),
        );
        cinput.set_completion_config_list(StoredConfig::list_fields().into_iter());

        let input = Component::new("input", cinput);
        let list_users_view: Component<'_, users_widget::UsersWidget> =
            Component::new("users_view", users_widget::UsersWidget::new());
        let topic_view: Component<'_, topic_widget::TopicWidget> =
            Component::new("topic_view", topic_widget::TopicWidget::new());
        //list_components.push()
        let log_dir = crate::project_path::ProjectPath::log_dir()
            .unwrap_or(std::env::current_dir().unwrap_or(std::path::Path::new("").to_path_buf()));
        let mut discuss_widget = discuss_widget::DiscussWidget::new(log_dir.clone());
        discuss_widget.set_current_channel(None, "Global");
        let messages_display = Component::new("messages", discuss_widget);
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
            logger: MessageLogger::new(log_dir),
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

    pub fn log(
        &mut self,
        connection_config: Option<&ConnectionConfig>,
        irc_model: Option<&IrcModel>,
        message: &MessageEvent,
    ) -> anyhow::Result<()> {
        if let Some(connection_config) = connection_config {
            self.logger
                .write_message(&connection_config.address, irc_model, message)
        } else {
            Ok(())
        }
    }

    pub fn flush_log(&mut self) -> std::io::Result<()> {
        self.logger.flush_checker()
    }

    fn update_input(
        &mut self,
        model: &mut Model,
        session: &mut Session,
        content: &str,
    ) -> Option<MessageEvent> {
        if let Some(parsed_message) = command::parse_command(content) {
            match parsed_message {
                command::ClientCommand::Connect => Some(MessageEvent::Connect(
                    ServerID::new(0), /*TODO: command should return the id */
                )),
                command::ClientCommand::Quit(message) => {
                    session.send_command_all_server(Command::Quit(message.clone()));

                    Some(MessageEvent::QuitAll(message))
                }
                command::ClientCommand::Help => Some(help()),
                command::ClientCommand::Nick(new_nick) => {
                    if let Err(e) =
                        session.send_command_current_server(Command::Nick(new_nick.clone()))
                    {
                        return Some(MessageEvent::from_error(e));
                    }

                    if let Some(id) = session.get_current_server_id() {
                        if !session.is_connected(id)
                            && let Err(e) = model.set_nickname(id, new_nick.clone())
                        {
                            tracing::error!(error = %e, "Impossible to save");
                        }
                    }

                    None
                }
                command::ClientCommand::Topic(topic) => {
                    if let Err(e) = session.send_command_topic(topic) {
                        return Some(MessageEvent::from_error(e));
                    }

                    None
                }
                command::ClientCommand::Spell(language) => {
                    Some(MessageEvent::SpellChecker(language))
                }
                command::ClientCommand::Join(channel) => {
                    if let Err(e) = session.send_command_join(channel) {
                        return Some(MessageEvent::from_error(e));
                    }

                    None
                }
                command::ClientCommand::Part(channel, reason) => {
                    if let Err(e) = session.send_command_part(channel, reason) {
                        return Some(MessageEvent::from_error(e));
                    }

                    None
                }
                command::ClientCommand::Action(content) => {
                    if let Err(e) = session.send_command_action(content.to_string()) {
                        return Some(MessageEvent::from_error(e));
                    }
                    if let Some(status) = session.get_current_status()
                        && let Some(status_channel) = status.channel
                    {
                        Some(MessageEvent::ActionMsg(
                            status.server_id,
                            status.nickname.to_string(),
                            status_channel.to_string(),
                            content,
                        ))
                    } else {
                        None
                    }
                }
                command::ClientCommand::PrivMSG(channel, content) => {
                    if let Err(e) = session.send_command_current_server(
                        clown_core::command::Command::PrivMsg(channel.clone(), content.clone()),
                    ) {
                        return Some(MessageEvent::from_error(e));
                    }
                    session.get_current_status().map(|v| {
                        MessageEvent::PrivMsg(v.server_id, v.nickname.to_string(), channel, content)
                    })
                }
                command::ClientCommand::Config(config_command_type, path, value) => {
                    match config_command_type {
                        command::ConfigCommand::Get => {
                            match model.get_config_value(&path, value.as_deref()) {
                                Ok(result) => Some(MessageEvent::AddMessageViewInfo(
                                    None,
                                    None,
                                    crate::message_irc::message_content::MessageKind::Info,
                                    format!("{path} {result}"),
                                )),
                                Err(e) => Some(MessageEvent::AddMessageViewInfo(
                                    None,
                                    None,
                                    crate::message_irc::message_content::MessageKind::Error,
                                    format!("{path} {e}"),
                                )),
                            }
                        }
                        command::ConfigCommand::Add | command::ConfigCommand::Set => {
                            if let Some(value) = value {
                                match model.set_config_value(&path, value) {
                                    Ok(_) => Some(MessageEvent::SettingsDidChange),
                                    Err(e) => Some(MessageEvent::AddMessageViewInfo(
                                        None,
                                        None,
                                        crate::message_irc::message_content::MessageKind::Error,
                                        format!("{} {}", path, e),
                                    )),
                                }
                            } else {
                                Some(MessageEvent::AddMessageViewInfo(
                                    None,
                                    None,
                                    crate::message_irc::message_content::MessageKind::Error,
                                    "No values to set".to_string(),
                                ))
                            }
                        }
                    }
                }
                command::ClientCommand::CloseBuffer(channel) => {
                    let status = session.get_current_status();

                    let server_id = status.as_ref().map(|s| s.server_id);

                    let channel =
                        channel.or_else(|| status.and_then(|s| s.channel.map(|v| v.to_string())));

                    if let Some(channel) = channel {
                        if channel.starts_with('#')
                            && let Err(e) = session.send_command_part(Some(channel.clone()), None)
                        {
                            return Some(MessageEvent::from_error(e));
                        }

                        Some(MessageEvent::CloseBuffer(server_id, channel))
                    } else {
                        None
                    }
                }

                command::ClientCommand::Unknown(command_name) => {
                    Some(MessageEvent::AddMessageViewInfo(
                        None,
                        None,
                        crate::message_irc::message_content::MessageKind::Error,
                        format!("Unknown command {}", command_name.unwrap_or_default()),
                    ))
                }
            }
        } else if let Some(cstatus) = session.get_current_status().map(|v| v.to_owned())
            && let Some(status_channel) = cstatus.channel
        {
            let content = content.to_string();
            if status_channel.eq_ignore_ascii_case(&cstatus.nickname) {
                Some(MessageEvent::PrivMsg(
                    cstatus.server_id,
                    cstatus.nickname.to_string(),
                    status_channel.to_string(),
                    content,
                ))
            } else {
                match session.send_command_current_server(clown_core::command::Command::PrivMsg(
                    status_channel.to_string(),
                    content.to_string(),
                )) {
                    Err(e) => Some(MessageEvent::from_error(e)),
                    _ => Some(MessageEvent::PrivMsg(
                        cstatus.server_id,
                        cstatus.nickname.to_string(),
                        status_channel.to_string(),
                        content,
                    )),
                }
            }
        } else {
            None
        }
    }

    fn handle_irc(
        &mut self,
        model: &mut Model,
        session: &mut Session,
        messages: &mut MessageQueue,
    ) {
        if model.running_state == RunningState::Start {
            model.running_state = RunningState::Running;

            for id in model.is_autojoin() {
                messages.push_message(MessageEvent::Connect(id));
            }
        } else {
            let mut to_delete = vec![];
            for (server_id, msg) in session.pull_all_server_error() {
                messages.push_message(MessageEvent::AddMessageViewInfo(
                    Some(server_id),
                    None,
                    crate::message_irc::message_content::MessageKind::Error,
                    msg,
                ));
                to_delete.push(server_id);

                messages.push_message_with_time(
                    MessageEvent::Connect(server_id),
                    std::time::Duration::from_secs(5),
                );
            }

            for server_id in to_delete {
                if session.is_irc_finished(server_id) {
                    session.clear_connection(server_id);
                }
            }
        }

        messages.push_message(MessageEvent::PullIRC);
    }

    fn handle_tick(
        &mut self,
        model: &mut Model,
        session: &mut Session,
        event: &Event,
        messages: &mut MessageQueue,
    ) {
        self.handle_irc(model, session, messages);
        if self.log_instant.elapsed() > std::time::Duration::from_secs(LOG_FLUSH_CHECK_TIMER) {
            if let Err(e) = self.flush_log() {
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

    fn update_pull_irc(
        &mut self,
        model: &mut Model,
        session: &mut Session,
        messages: &mut MessageQueue,
    ) {
        let mut server_to_init = vec![];
        for (server_id, recieved) in session.pull_all_server_message() {
            let reply = recieved.reply();
            let source = recieved.source().map(|v| v.to_string());

            debug!("server_id : {:?}, {:?}", server_id, recieved);
            //log_info_sync(format!("{reply:?}\n").as_str());
            match reply {
                Response::Cmd(command) => match command {
                    Command::PrivMsg(target, content) => {
                        if let Some(source) = source {
                            if content.starts_with("\x01ACTION") {
                                if let Some(parsed_content) = content.get(8..content.len() - 1) {
                                    messages.push_message(MessageEvent::ActionMsg(
                                        server_id,
                                        source,
                                        target,
                                        parsed_content.to_string(),
                                    ));
                                }
                            } else {
                                messages.push_message(MessageEvent::PrivMsg(
                                    server_id, source, target, content,
                                ));
                            }
                        } else {
                            tracing::error!(error = %MessageError::MissingSource, "PrivMSG");
                        }
                    }
                    Command::Nick(new_user) => {
                        if let Some(source) = source
                            && let Some(nickname) = model.get_nickname(server_id)
                        {
                            if source.eq_ignore_ascii_case(nickname) {
                                if let Err(e) = model.set_nickname(server_id, new_user.clone()) {
                                    tracing::error!(error = %e, "Impossible to save");
                                }
                            }

                            messages.push_message(MessageEvent::ReplaceUser(
                                server_id, source, new_user,
                            ));
                        } else {
                            tracing::error!(error = %MessageError::MissingSource, "Nick");
                        }
                    }
                    Command::Notice(target, message) => {
                        //Display a notice directly to the user current channel
                        if let Some(source) = source {
                            messages.push_message(MessageEvent::Notice(
                                server_id, target, source, message,
                            ));
                        }
                    }
                    Command::Topic(channel, topic) => {
                        messages.push_message(MessageEvent::SetTopic(
                            server_id, source, channel, topic,
                        ));
                    }
                    Command::Quit(reason) => {
                        if let Some(source) = source {
                            messages.push_message(MessageEvent::Quit(server_id, source, reason));
                        } else {
                            tracing::error!(error = %MessageError::MissingSource, "Quit");
                        }
                    }
                    Command::Part(channel, _reason) => {
                        if let Some(source) = source {
                            messages.push_message(MessageEvent::Part(
                                server_id,
                                channel.to_string(),
                                source,
                            ));
                        } else {
                            tracing::error!(error = %MessageError::MissingSource, "Part");
                        }
                    }
                    Command::Join(channel) => {
                        if let Some(source) = source {
                            //Create a new 'user' as IRC-Server
                            messages.push_message(MessageEvent::Join(
                                server_id,
                                channel.clone(),
                                source.clone(),
                            ));

                            messages.push_message(MessageEvent::SelectChannel(
                                Some(server_id),
                                channel.clone(),
                            ));
                        } else {
                            tracing::error!(error = %MessageError::MissingSource, "Join");
                        }
                    }
                    Command::Error(err) => {
                        messages.push_message(MessageEvent::AddMessageViewInfo(
                            Some(server_id),
                            None,
                            crate::message_irc::message_content::MessageKind::Error,
                            err,
                        ));
                        messages.push_message(MessageEvent::DisConnect(server_id))
                    }
                    Command::Unknown(content) => {
                        messages.push_message(MessageEvent::AddMessageViewInfo(
                            Some(server_id),
                            None,
                            crate::message_irc::message_content::MessageKind::Error,
                            content,
                        ));
                    }
                    _ => {}
                },
                Response::Rpl(reply) => match reply {
                    ResponseNumber::Welcome(content) => {
                        server_to_init.push(server_id);

                        messages.push_message(MessageEvent::AddMessageViewInfo(
                            Some(server_id),
                            source.clone(),
                            crate::message_irc::message_content::MessageKind::Normal,
                            content,
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
                        messages.push_message(MessageEvent::AddMessageViewInfo(
                            Some(server_id),
                            source.clone(),
                            crate::message_irc::message_content::MessageKind::Normal,
                            content,
                        ));
                    }
                    ResponseNumber::NameReply(_symbol, channel, list_users) => {
                        messages.push_message(MessageEvent::UpdateUsers(
                            server_id, channel, list_users,
                        ));
                    }
                    ResponseNumber::Topic(channel, topic) => {
                        messages
                            .push_message(MessageEvent::SetTopic(server_id, None, channel, topic));
                    }
                    ResponseNumber::Err(_, content) => {
                        messages.push_message(MessageEvent::AddMessageViewInfo(
                            Some(server_id),
                            None,
                            crate::message_irc::message_content::MessageKind::Error,
                            content,
                        ));
                    }
                    _ => {
                        //tracing::error!(error = %MessageError::UnknownMessage);
                    }
                },
                Response::Unknown(content) => {
                    messages.push_message(MessageEvent::AddMessageViewInfo(
                        Some(server_id),
                        None,
                        crate::message_irc::message_content::MessageKind::Error,
                        content,
                    ));
                }
            };
        }

        for id in server_to_init {
            session.reset_retry();
            if model.is_autojoin_by_id(id) {
                for channel in model.get_channels(id) {
                    if let Err(e) = session
                        .send_command(id, clown_core::command::Command::Join(channel.to_string()))
                    {
                        messages.push_message(e.into());
                    }
                }
            }
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
    fn view(&mut self, model: &mut Model, session: &mut Session, frame: &mut Frame<'_>) {
        if self.need_redraw {
            self.need_redraw = false;
        }
        // Create layout
        let main_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(if model.is_topic_ui_enabled() { 1 } else { 0 }), // Topic area
                Constraint::Percentage(100), // Messages area
                Constraint::Length(2),       // Input area
            ])
            .split(frame.area());

        if let Some(message_area_layout) = main_layout.get(1) {
            let top_layout = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Percentage(100), // Messages
                    Constraint::Min(if model.is_users_ui_enabled() { 15 } else { 0 }), // List Users
                ])
                .split(*message_area_layout);

            if let Some(message_area) = top_layout.first() {
                self.messages_display
                    .render(model, Some(&session.model), frame, *message_area);
                self.tooltip_widget
                    .render(model, Some(&session.model), frame, *message_area);
            }

            if let Some(list_users) = top_layout.get(1) {
                self.list_users_view
                    .render(model, Some(&session.model), frame, *list_users);
            }
        }

        // Render widgets
        if let Some(input_area) = main_layout.get(2) {
            self.input
                .render(model, Some(&session.model), frame, *input_area);
        }

        if let Some(topic_area) = main_layout.first() {
            self.topic_view
                .render(model, Some(&session.model), frame, *topic_area);
        }
    }

    fn handle_event(
        &mut self,
        model: &mut Model,
        session: &mut Session,
        event: &Event,
        messages: &mut MessageQueue,
    ) {
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
                self.handle_tick(model, session, event, messages);
            }
            _ => {}
        };
    }

    fn update(
        &mut self,
        model: &mut Model,
        session: &mut Session,
        msg: MessageEvent,
        messages: &mut MessageQueue,
    ) {
        match &msg {
            MessageEvent::MessageInput(content) => {
                for m in content.split(['\r', '\n']).filter(|s| !s.is_empty()) {
                    if let Some(v) = self.update_input(model, session, m) {
                        messages.push_message(v);
                    }
                }
                return; // Usually, we don't need to propagate raw input to children here
            }
            #[allow(clippy::print_stdout)]
            MessageEvent::Bel => {
                println!("{}", 0x07 as char);
                return;
            }
            MessageEvent::OpenWeb(url) => {
                if let Err(e) = open::that(url) {
                    error!("Try to open {}, {}", url, e);
                }
                return;
            }
            MessageEvent::Connect(server_id) => {
                let addr = model.get_address(*server_id).unwrap_or("No address");

                messages.push_message(MessageEvent::JoinServer(*server_id));

                messages.push_message(MessageEvent::AddMessageViewInfo(
                    Some(*server_id),
                    None,
                    crate::message_irc::message_content::MessageKind::Info,
                    format!("Try to connect to {}...", addr),
                ));
                let server_name = model.get_name(*server_id).to_string();
                messages.push_message(MessageEvent::SelectChannel(
                    Some(*server_id),
                    server_name.clone(),
                ));

                if let Some(conn_cfg) = model.get_connection_config(*server_id)
                    && let Some(login_cfg) = model.get_login_config(*server_id)
                {
                    if session.is_irc_finished(*server_id) {
                        if let Err(e) = session.init_connection(*server_id, conn_cfg, login_cfg) {
                            tracing::error!(error =%e);
                            messages.push_message(MessageEvent::AddMessageViewInfo(
                                Some(*server_id),
                                None,
                                crate::message_irc::message_content::MessageKind::Error,
                                e.to_string(),
                            ));
                        } else {
                            let nick = model.get_nickname(*server_id).unwrap_or("No Nick");
                            session.init_irc_model(nick.to_string(), *server_id, server_name);
                        }
                    } else {
                        messages.push_message(MessageEvent::AddMessageViewInfo(
                            Some(*server_id),
                            None,
                            crate::message_irc::message_content::MessageKind::Error,
                            format!("Already connected to {}...", addr),
                        ));
                    }
                } else {
                    messages.push_message(MessageEvent::AddMessageViewInfo(
                        Some(*server_id),
                        None,
                        crate::message_irc::message_content::MessageKind::Error,
                        format!("Cannot connect to {}...", addr),
                    ));
                }
                return;
            }
            MessageEvent::DisConnect(server_id) => {
                if !session.is_irc_finished(*server_id) {
                    if let Err(e) =
                        session.send_command(*server_id, clown_core::command::Command::Quit(None))
                    {
                        messages.push_message(e.into());
                    }
                } else {
                    messages.push_message(MessageEvent::AddMessageViewInfo(
                        Some(*server_id),
                        None,
                        crate::message_irc::message_content::MessageKind::Info,
                        "Disconnected".to_string(),
                    ));
                }
            }
            MessageEvent::PullIRC => {
                self.update_pull_irc(model, session, messages);
                return;
            }
            MessageEvent::QuitAll(reason) => {
                for id in session.iter_valid_connection_id() {
                    if let Some(nickname) = model.get_nickname(id)
                        && let Err(e) = self.log(
                            model.get_connection_config(id).as_ref(),
                            Some(&session.model),
                            &MessageEvent::Quit(id, nickname.to_string(), reason.clone()),
                        )
                    {
                        tracing::error!(error = %e, "Cannot write logs");
                    }
                }
                model.running_state = RunningState::Done;
            }
            // Handle Logging for IRC events
            MessageEvent::ActionMsg(id, ..)
            | MessageEvent::Join(id, ..)
            | MessageEvent::JoinServer(id, ..)
            | MessageEvent::Part(id, ..)
            | MessageEvent::Quit(id, ..)
            | MessageEvent::ReplaceUser(id, ..)
            | MessageEvent::PrivMsg(id, ..)
            | MessageEvent::UpdateUsers(id, ..)
            | MessageEvent::SetTopic(id, ..) => {
                if let Err(e) = self.log(
                    model.get_connection_config(*id).as_ref(),
                    Some(&session.model),
                    &msg,
                ) {
                    tracing::error!(error = %e, "Cannot write logs");
                }
            }
            _ => {}
        }

        for child in self.children().iter_mut() {
            if let Some(new_msg) = child.handle_actions(model, Some(&session.model), &msg) {
                messages.push_message(new_msg);
            }
        }
        session.handle_action(&msg);
    }
}
