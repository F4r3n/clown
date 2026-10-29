use super::servers_messages::{ServersMessages, WordRange};
use crate::component::Draw;
use crate::irc_view::discuss::message_collection::MessageViewer;
use crate::message_event::MessageEvent;
use crate::state::irc_model::IrcModel;
use crate::state::irc_name::IrcName;
use crate::state::server_id::ServerID;
use clown_message_render::message_content::MessageContent;
use crossterm::event::KeyCode;
use crossterm::event::MouseButton;
use ratatui::{Frame, layout::Rect};
use std::time::Duration;

#[derive(Debug)]
struct Hovered {
    time: std::time::Instant,
    range: WordRange,
}

impl Hovered {
    pub fn new(range: WordRange) -> Self {
        Self {
            range,
            time: std::time::Instant::now(),
        }
    }
}

#[derive(Debug)]
pub struct DiscussWidget {
    message_viewer: MessageViewer,
    current_channel: IrcName,
    current_server_id: Option<ServerID>,
    last_hovered: Option<Hovered>,
    display_time: bool,
}

impl DiscussWidget {
    pub fn new() -> Self {
        Self {
            current_channel: IrcName::default(),
            current_server_id: None,
            message_viewer: MessageViewer::new(),
            last_hovered: None,
            display_time: true,
        }
    }

    pub fn set_current_channel(&mut self, server_id: Option<ServerID>, channel: IrcName) {
        self.current_channel = channel;
        self.current_server_id = server_id;
        self.message_viewer.reset_scroll();
    }

    #[cfg(any(test, feature = "bench"))]
    #[allow(dead_code)]
    pub fn set_scroll_offset(&mut self, offset: usize) {
        self.message_viewer.set_scroll_offset(offset);
    }

    fn get_range_from_mouse(
        &self,
        messages: &ServersMessages,
        row: u16,
        col: u16,
    ) -> Option<WordRange> {
        messages
            .get_messages(self.current_server_id, &self.current_channel)
            .and_then(|messages| self.message_viewer.get_range_from_mouse(messages, row, col))
    }

    #[cfg(any(test, feature = "bench"))]
    #[allow(dead_code)]
    pub fn total_lines(&self, messages: &ServersMessages) -> usize {
        messages
            .get_messages(self.current_server_id, &self.current_channel)
            .map(|messages| self.message_viewer.total_lines(messages))
            .unwrap_or(0)
    }

    pub fn has_message(
        &self,
        messages: &ServersMessages,
        server_id: Option<ServerID>,
        channel: &IrcName,
    ) -> bool {
        messages.has_messages(server_id, channel)
    }

    fn add_line_scroll(&mut self) {
        self.message_viewer.add_line_scroll();
    }

    pub fn add_line(
        &mut self,
        messages: &mut ServersMessages,
        server_id: Option<ServerID>,
        channel: &IrcName,
        in_message: MessageContent,
    ) {
        messages.add_message(server_id, channel, in_message);
        if channel.eq(&self.current_channel) && server_id == self.current_server_id {
            self.add_line_scroll();
        }

        self.message_viewer.ask_redraw();
    }

    pub fn add_server_group(
        &mut self,
        messages: &mut ServersMessages,
        server_id: Option<ServerID>,
        server_address: Option<String>,
    ) {
        messages.add_server_group(server_id, server_address);
    }

    pub fn add_line_current(&mut self, messages: &mut ServersMessages, in_message: MessageContent) {
        messages.add_message(self.current_server_id, &self.current_channel, in_message);
        self.add_line_scroll();

        self.message_viewer.ask_redraw();
    }

    //How many lines I can scroll up
    fn possible_scroll_up(&mut self, messages: &ServersMessages, offset: usize) -> usize {
        if let Some(messages) = messages.get_messages(self.current_server_id, &self.current_channel)
        {
            self.message_viewer.possible_scroll_up(messages, offset)
        } else {
            0
        }
    }

    fn scroll_up_no_check(&mut self) {
        self.message_viewer.scroll_up_no_check(1)
    }

    fn scroll_up_logs(&mut self, current_nick: &IrcName, messages: &mut ServersMessages) {
        if self.possible_scroll_up(messages, 1) > 0 || self.read_log(current_nick, messages, 1) > 0
        {
            self.scroll_up_no_check();
        }
    }

    fn read_log(
        &mut self,
        current_nick: &IrcName,
        messages: &mut ServersMessages,
        number_lines: usize,
    ) -> usize {
        if let Some(server_id) = self.current_server_id {
            //cannot scroll up, maybe open the logs
            messages.open_log(server_id, &self.current_channel);
            match messages.read_log(current_nick, number_lines, server_id, &self.current_channel) {
                Ok(v) => v,
                Err(e) => {
                    tracing::error!("Cannot read logs: {e}");
                    0
                }
            }
        } else {
            0
        }
    }

    fn scroll_page_up(&mut self, messages: &mut ServersMessages) -> usize {
        if let Some(messages) = messages.get_messages(self.current_server_id, &self.current_channel)
        {
            self.message_viewer.scroll_page_up(messages)
        } else {
            0
        }
    }

    fn scroll_page_up_logs(&mut self, current_nick: &IrcName, messages: &mut ServersMessages) {
        let offset = self.scroll_page_up(messages);
        if offset < self.message_viewer.get_max_visible_height() {
            let log_offset = self.read_log(
                current_nick,
                messages,
                self.message_viewer
                    .get_max_visible_height()
                    .saturating_sub(offset),
            );
            if log_offset > 0 {
                self.message_viewer.scroll_up_no_check(log_offset);
            }
        }
    }

    fn scroll_page_down(&mut self) {
        self.message_viewer
            .scroll_down_no_check(self.message_viewer.get_max_visible_height());
    }

    fn scroll_down(&mut self) {
        self.message_viewer.scroll_down_no_check(1);
    }

    fn get_current_nick<'b>(&self, irc_model: &'b IrcModel) -> Option<&'b IrcName> {
        self.current_server_id
            .and_then(|v| irc_model.get_server(v).map(|v| v.get_current_nick()))
    }
}

impl Draw for DiscussWidget {
    fn render(&mut self, ctx: &mut crate::state::context::Ctx, frame: &mut Frame<'_>, area: Rect) {
        match ctx
            .messages
            .get_messages(self.current_server_id, &self.current_channel)
        {
            Some(messages) => self.message_viewer.render(
                messages,
                |s| ctx.model.get_color(s),
                self.display_time,
                frame,
                area,
            ),
            None => self.message_viewer.render(
                &Vec::new(),
                |s| ctx.model.get_color(s),
                self.display_time,
                frame,
                area,
            ),
        }
    }
}

impl crate::component::EventHandler for DiscussWidget {
    fn get_area(&self) -> Rect {
        self.message_viewer.get_area()
    }
    fn need_redraw(&self) -> bool {
        self.message_viewer.need_redraw()
    }

    fn handle_actions(
        &mut self,
        ctx: &mut crate::state::context::Ctx,
        event: &MessageEvent,
    ) -> Option<MessageEvent> {
        match event {
            MessageEvent::AddMessageViewInfo(server_id, channel, kind, in_content) => {
                for content in in_content.split('\n') {
                    if let Some(message) =
                        MessageContent::from_kind(*kind, channel.clone(), content.to_string())
                    {
                        if let Some(server_id) = server_id {
                            if let Some(server_name) = ctx
                                .session
                                .model
                                .get_server_name_from_channel(*server_id, channel.as_deref())
                            {
                                self.add_line(
                                    &mut ctx.messages,
                                    Some(*server_id),
                                    &IrcName::new(server_name),
                                    message,
                                );
                            }
                        } else {
                            self.add_line_current(&mut ctx.messages, message);
                        }
                    }
                }

                None
            }
            MessageEvent::SetTopic(server_id, source, channel, content) => {
                if let Some(source) = source {
                    let data = format!(
                        "{} has changed topic for {} to \"{}\"",
                        source, channel, content
                    );
                    self.add_line(
                        &mut ctx.messages,
                        Some(*server_id),
                        channel,
                        MessageContent::info(data),
                    );
                }

                None
            }
            MessageEvent::Quit(server_id, user, reason) => {
                if let Some(irc_server) = ctx.session.model.get_server(*server_id) {
                    for channel in irc_server.get_all_joined_channel(user) {
                        self.add_line(
                            &mut ctx.messages,
                            Some(*server_id),
                            channel,
                            MessageContent::info(
                                reason
                                    .as_ref()
                                    .map(|v| format!("{} has quit: {}", user, v))
                                    .unwrap_or_else(|| format!("{} has quit", user)),
                            ),
                        );
                    }

                    if self.has_message(&ctx.messages, Some(*server_id), user) {
                        self.add_line(
                            &mut ctx.messages,
                            Some(*server_id),
                            user,
                            MessageContent::info(
                                reason
                                    .as_ref()
                                    .map(|v| format!("{} has quit: {}", user, v))
                                    .unwrap_or_else(|| format!("{} has quit", user)),
                            ),
                        );
                    }
                }

                None
            }

            MessageEvent::Part(server_id, channel, user) => {
                let msg = format!("{} has left {}", user, channel);
                if self.has_message(&ctx.messages, Some(*server_id), user) {
                    self.add_line(
                        &mut ctx.messages,
                        Some(*server_id),
                        user,
                        MessageContent::info(msg.clone()),
                    );
                }
                self.add_line(
                    &mut ctx.messages,
                    Some(*server_id),
                    channel,
                    MessageContent::info(msg),
                );

                None
            }
            MessageEvent::PrivMsg(server_id, source, target, content) => {
                if let Some(irc_server) = ctx.session.model.get_server(*server_id)
                    && let Some(current_channel) = irc_server.get_current_channel()
                {
                    let target = irc_server.get_target(source, target);

                    if source == irc_server.get_current_nick() && target != current_channel {
                        self.add_line(
                            &mut ctx.messages,
                            Some(*server_id),
                            current_channel,
                            MessageContent::privmsg(target.to_string(), content.clone()),
                        );
                    }

                    let is_highlight = MessageContent::should_highlight(
                        irc_server.get_current_nick().as_str(),
                        content,
                    );

                    self.add_line(
                        &mut ctx.messages,
                        Some(*server_id),
                        target,
                        if is_highlight {
                            MessageContent::highlight(Some(source.to_string()), content.clone())
                        } else {
                            MessageContent::message(Some(source.to_string()), content.clone())
                        },
                    );
                    if is_highlight {
                        Some(MessageEvent::Bel)
                    } else {
                        None
                    }
                } else {
                    None
                }
            }
            MessageEvent::Notice(server_id, source, target, content) => {
                if let Some(irc_server) = ctx.session.model.get_server(*server_id) {
                    let target = irc_server.get_target(source, target);

                    self.add_line(
                        &mut ctx.messages,
                        Some(*server_id),
                        target,
                        MessageContent::notice(Some(source.to_string()), content.clone()),
                    );
                }

                None
            }
            MessageEvent::ActionMsg(server_id, source, target, content) => {
                if let Some(irc_server) = ctx.session.model.get_server(*server_id) {
                    let target = irc_server.get_target(source, target);

                    self.add_line(
                        &mut ctx.messages,
                        Some(*server_id),
                        target,
                        MessageContent::action(source.to_string(), content.clone()),
                    );
                }

                None
            }
            MessageEvent::JoinServer(server_id) => {
                self.add_server_group(
                    &mut ctx.messages,
                    Some(*server_id),
                    ctx.model
                        .get_connection_config(*server_id)
                        .as_ref()
                        .map(|v| v.address.clone()),
                );

                None
            }
            MessageEvent::Join(server_id, channel, source) => {
                if let Some(irc_server) = ctx.session.model.get_server(*server_id) {
                    let main = irc_server.is_main_user(source);
                    self.add_line(
                        &mut ctx.messages,
                        Some(*server_id),
                        channel,
                        if main {
                            MessageContent::info(format!("You joined the channel {}", channel))
                        } else {
                            MessageContent::info(format!("{} has joined", source))
                        },
                    );
                }
                None
            }
            MessageEvent::SettingsDidChange => {
                self.display_time = ctx.model.get_discuss_config().left_bar.time;

                None
            }
            MessageEvent::ReplaceUser(server_id, old, new) => {
                if let Some(irc_server) = ctx.session.model.get_server(*server_id) {
                    for channel in irc_server.get_all_joined_channel(old) {
                        self.add_line(
                            &mut ctx.messages,
                            Some(*server_id),
                            channel,
                            MessageContent::info(format!(
                                "{} has changed their nickname to {}",
                                old, new
                            )),
                        );
                    }

                    ctx.messages.rename(Some(*server_id), old, new);
                    if self.current_channel == *old {
                        self.current_channel = new.clone();
                        self.message_viewer.invalidate_cache();
                    }
                }
                None
            }
            MessageEvent::SelectChannel(server_id, channel) => {
                self.set_current_channel(*server_id, channel.clone());
                None
            }
            _ => None,
        }
    }

    fn handle_events(
        &mut self,
        ctx: &mut crate::state::context::Ctx,
        event: &crate::event_handler::Event,
    ) -> Option<MessageEvent> {
        if let crate::event_handler::Event::Crossterm(cross) = &event {
            match cross {
                crossterm::event::Event::Resize(_x, _y) => {
                    self.message_viewer.ask_redraw();
                    None
                }
                crossterm::event::Event::Mouse(mouse_event) => match mouse_event.kind {
                    crossterm::event::MouseEventKind::ScrollDown => {
                        self.scroll_down();
                        None
                    }
                    crossterm::event::MouseEventKind::ScrollUp => {
                        let no_nick = IrcName::default();
                        let nick = self
                            .get_current_nick(&ctx.session.model)
                            .unwrap_or(&no_nick);
                        self.scroll_up_logs(nick, &mut ctx.messages);
                        None
                    }
                    crossterm::event::MouseEventKind::Down(button) => {
                        if button == MouseButton::Left {
                            if let Some(range) = self.get_range_from_mouse(
                                &ctx.messages,
                                mouse_event.row,
                                mouse_event.column,
                            ) {
                                ctx.messages
                                    .get_url_from_range(
                                        self.current_server_id,
                                        &self.current_channel,
                                        &range,
                                    )
                                    .map(MessageEvent::OpenWeb)
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    }
                    #[cfg(feature = "website-preview")]
                    crossterm::event::MouseEventKind::Moved => {
                        if let Some(range) = self.get_range_from_mouse(
                            &ctx.messages,
                            mouse_event.row,
                            mouse_event.column,
                        ) {
                            let mut should_clear = false;
                            let result = if let Some(last_hovered) = &mut self.last_hovered
                                && range.eq(&last_hovered.range)
                            {
                                if std::time::Instant::now().duration_since(last_hovered.time)
                                    > Duration::from_secs(2)
                                {
                                    if let Some(url) = ctx.messages.get_url_from_range(
                                        self.current_server_id,
                                        &self.current_channel,
                                        &last_hovered.range,
                                    ) {
                                        should_clear = true;
                                        Some(MessageEvent::HoverURL(url))
                                    } else {
                                        //Not an url, no need to wait for next round
                                        should_clear = true;
                                        None
                                    }
                                } else {
                                    None
                                }
                            } else {
                                self.last_hovered = Some(Hovered::new(range));
                                None
                            };

                            if should_clear {
                                self.last_hovered = None;
                            }

                            result
                        } else {
                            self.last_hovered = None;
                            None
                        }
                    }
                    _ => None,
                },
                crossterm::event::Event::Key(key_event) => match key_event.code {
                    KeyCode::PageUp => {
                        let no_nick = IrcName::default();
                        let nick = self
                            .get_current_nick(&ctx.session.model)
                            .unwrap_or(&no_nick);
                        self.scroll_page_up_logs(nick, &mut ctx.messages);
                        None
                    }
                    KeyCode::PageDown => {
                        self.scroll_page_down();
                        None
                    }

                    _ => None,
                },
                _ => None,
            }
        } else {
            match event {
                crate::event_handler::Event::Tick => {
                    let mut should_clear = false;
                    let result = if let Some(last_hovered) = &mut self.last_hovered {
                        if std::time::Instant::now().duration_since(last_hovered.time)
                            > Duration::from_secs(2)
                        {
                            if let Some(url) = ctx.messages.get_url_from_range(
                                self.current_server_id,
                                &self.current_channel,
                                &last_hovered.range,
                            ) {
                                should_clear = true;
                                Some(MessageEvent::HoverURL(url))
                            } else {
                                //Not an url, no need to wait for next round
                                should_clear = true;
                                None
                            }
                        } else {
                            None
                        }
                    } else {
                        None
                    };

                    if should_clear {
                        self.last_hovered = None;
                    }

                    result
                }
                _ => None,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::component::EventHandler;
    use crate::state::context::Ctx;
    use crate::state::model::Model;
    use crate::state::session::Session;
    use ratatui::{Terminal, backend::TestBackend};

    const SERVER_ID: ServerID = ServerID::new(0);
    const NICK: &str = "clowny";
    const CHANNEL: &str = "#rust";

    // Same order as the main view: widgets first, then the model.
    fn dispatch(
        widget: &mut DiscussWidget,
        ctx: &mut Ctx,
        event: MessageEvent,
    ) -> Option<MessageEvent> {
        let result = widget.handle_actions(ctx, &event);
        ctx.session.handle_action(&event);
        result
    }

    fn join(widget: &mut DiscussWidget, ctx: &mut Ctx, channel: &str, user: &str) {
        dispatch(
            widget,
            ctx,
            MessageEvent::Join(SERVER_ID, channel.into(), user.into()),
        );
    }

    fn privmsg(
        widget: &mut DiscussWidget,
        ctx: &mut Ctx,
        source: &str,
        target: &str,
        content: &str,
    ) -> Option<MessageEvent> {
        dispatch(
            widget,
            ctx,
            MessageEvent::PrivMsg(SERVER_ID, source.into(), target.into(), content.to_string()),
        )
    }

    // Connected to one server, joined and looking at CHANNEL.
    fn setup() -> (Ctx, DiscussWidget) {
        let mut ctx = Ctx {
            session: Session::new(1),
            model: Model::new_empty_config(),
            messages: ServersMessages::new(std::path::PathBuf::new()),
        };
        ctx.session
            .model
            .init_server(SERVER_ID, "irc.test".to_string(), NICK.to_string());

        let mut widget = DiscussWidget::new();
        dispatch(&mut widget, &mut ctx, MessageEvent::JoinServer(SERVER_ID));
        join(&mut widget, &mut ctx, CHANNEL, NICK);
        dispatch(
            &mut widget,
            &mut ctx,
            MessageEvent::SelectChannel(Some(SERVER_ID), CHANNEL.into()),
        );

        (ctx, widget)
    }

    fn contents(ctx: &Ctx, channel: &str) -> Vec<String> {
        ctx.messages
            .get_messages(Some(SERVER_ID), &channel.into())
            .map(|m| m.iter().map(|c| c.get_content().to_string()).collect())
            .unwrap_or_default()
    }

    fn last(ctx: &Ctx, channel: &str) -> Option<MessageContent> {
        ctx.messages
            .get_messages(Some(SERVER_ID), &channel.into())
            .and_then(|m| m.iter().last().cloned())
    }

    #[test]
    fn test_join_is_shown_in_channel() {
        let (mut ctx, mut widget) = setup();
        join(&mut widget, &mut ctx, CHANNEL, "alice");

        assert_eq!(
            contents(&ctx, CHANNEL),
            vec![
                format!("You joined the channel {CHANNEL}"),
                "alice has joined".to_string()
            ]
        );
    }

    #[test]
    fn test_privmsg_in_channel() {
        let (mut ctx, mut widget) = setup();

        assert_eq!(
            privmsg(&mut widget, &mut ctx, "alice", CHANNEL, "hello"),
            None
        );

        let message = last(&ctx, CHANNEL).expect("message in channel");
        assert_eq!(message.get_content(), "hello");
        assert_eq!(message.get_source(), Some("alice"));
        assert!(!message.is_highlighted());
    }

    #[test]
    fn test_privmsg_to_me_goes_to_query_buffer() {
        let (mut ctx, mut widget) = setup();
        let before = contents(&ctx, CHANNEL);

        privmsg(&mut widget, &mut ctx, "alice", NICK, "psst");

        assert_eq!(contents(&ctx, "alice"), vec!["psst".to_string()]);
        assert_eq!(contents(&ctx, CHANNEL), before);
    }

    #[test]
    fn test_own_privmsg_is_echoed_in_current_channel() {
        let (mut ctx, mut widget) = setup();

        privmsg(&mut widget, &mut ctx, NICK, "bob", "yo");

        assert_eq!(contents(&ctx, "bob"), vec!["yo".to_string()]);
        let echo = last(&ctx, CHANNEL).expect("echo in current channel");
        assert_eq!(echo.get_content(), "yo");
        assert_eq!(echo.get_source(), Some(">bob<"));
    }

    #[test]
    fn test_highlight_rings_bell() {
        let (mut ctx, mut widget) = setup();

        assert_eq!(
            privmsg(&mut widget, &mut ctx, "alice", CHANNEL, "hey clowny!"),
            Some(MessageEvent::Bel)
        );
        assert!(last(&ctx, CHANNEL).is_some_and(|m| m.is_highlighted()));
    }

    #[test]
    fn test_part_is_shown_in_channel_and_existing_query() {
        let (mut ctx, mut widget) = setup();
        join(&mut widget, &mut ctx, CHANNEL, "alice");
        join(&mut widget, &mut ctx, CHANNEL, "bob");

        // No query with bob: only the channel gets the line
        dispatch(
            &mut widget,
            &mut ctx,
            MessageEvent::Part(SERVER_ID, CHANNEL.into(), "bob".into()),
        );
        let msg = format!("bob has left {CHANNEL}");
        assert_eq!(
            last(&ctx, CHANNEL).map(|m| m.get_content().to_string()),
            Some(msg)
        );
        assert!(!widget.has_message(&ctx.messages, Some(SERVER_ID), &"bob".into()));

        // Query open with alice: both get the line
        privmsg(&mut widget, &mut ctx, "alice", NICK, "psst");
        dispatch(
            &mut widget,
            &mut ctx,
            MessageEvent::Part(SERVER_ID, CHANNEL.into(), "alice".into()),
        );
        let msg = format!("alice has left {CHANNEL}");
        assert_eq!(
            last(&ctx, CHANNEL).map(|m| m.get_content().to_string()),
            Some(msg.clone())
        );
        assert_eq!(contents(&ctx, "alice"), vec!["psst".to_string(), msg]);
    }

    #[test]
    fn test_quit_is_shown_in_every_joined_channel() {
        let (mut ctx, mut widget) = setup();
        join(&mut widget, &mut ctx, "#clown", NICK);
        join(&mut widget, &mut ctx, "#other", NICK);
        join(&mut widget, &mut ctx, CHANNEL, "alice");
        join(&mut widget, &mut ctx, "#clown", "alice");
        let other_before = contents(&ctx, "#other");

        dispatch(
            &mut widget,
            &mut ctx,
            MessageEvent::Quit(SERVER_ID, "alice".into(), Some("bye".to_string())),
        );

        for channel in [CHANNEL, "#clown"] {
            assert_eq!(
                last(&ctx, channel).map(|m| m.get_content().to_string()),
                Some("alice has quit: bye".to_string()),
                "{channel}"
            );
        }
        assert_eq!(contents(&ctx, "#other"), other_before);
    }

    #[test]
    fn test_nick_change_renames_query_and_follows_current_channel() {
        let (mut ctx, mut widget) = setup();
        join(&mut widget, &mut ctx, CHANNEL, "alice");
        privmsg(&mut widget, &mut ctx, "alice", NICK, "psst");
        dispatch(
            &mut widget,
            &mut ctx,
            MessageEvent::SelectChannel(Some(SERVER_ID), "alice".into()),
        );

        dispatch(
            &mut widget,
            &mut ctx,
            MessageEvent::ReplaceUser(SERVER_ID, "alice".into(), "alice2".into()),
        );

        assert_eq!(widget.current_channel, IrcName::new("alice2"));
        assert_eq!(contents(&ctx, "alice2"), vec!["psst".to_string()]);
        assert!(!widget.has_message(&ctx.messages, Some(SERVER_ID), &"alice".into()));
        assert_eq!(
            last(&ctx, CHANNEL).map(|m| m.get_content().to_string()),
            Some("alice has changed their nickname to alice2".to_string())
        );
    }

    #[test]
    fn test_nick_change_keeps_scroll_position() {
        let (mut ctx, mut widget) = setup();
        privmsg(&mut widget, &mut ctx, "alice", NICK, "psst");
        dispatch(
            &mut widget,
            &mut ctx,
            MessageEvent::SelectChannel(Some(SERVER_ID), "alice".into()),
        );
        widget.set_scroll_offset(3);

        dispatch(
            &mut widget,
            &mut ctx,
            MessageEvent::ReplaceUser(SERVER_ID, "alice".into(), "alice2".into()),
        );

        assert_eq!(widget.message_viewer.get_scroll_offset(), 3);
    }

    #[test]
    fn test_render_channel_without_messages() {
        let (mut ctx, mut widget) = setup();
        widget.set_current_channel(Some(SERVER_ID), "#empty".into());
        assert!(
            ctx.messages
                .get_messages(Some(SERVER_ID), &"#empty".into())
                .is_none()
        );

        let mut terminal = Terminal::new(TestBackend::new(80, 10)).expect("terminal");
        terminal
            .draw(|frame| widget.render(&mut ctx, frame, frame.area()))
            .expect("draw");

        assert!(!widget.need_redraw());
    }
}
