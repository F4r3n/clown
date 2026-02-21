use crate::{component::Draw, irc_view::color_user::nickname_color};

use crossterm::event::KeyModifiers;
use ratatui::{
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{List, ListItem},
};

#[derive(Debug)]
struct RegisteredSection {
    name: String,
    id: usize,
    server_id: Option<usize>,
    color: ratatui::style::Color,
    highlight: bool,
}

impl RegisteredSection {
    pub fn new(name: String, id: usize, server_id: Option<usize>) -> Self {
        Self {
            server_id,
            color: nickname_color(&name),
            name,
            id,
            highlight: false,
        }
    }
}

#[derive(Debug)]
struct Section {
    pub section_info: RegisteredSection,
    pub order_user: Vec<String>,
}

impl Section {
    fn new(channel_name: String, id: usize, server_id: Option<usize>) -> Self {
        Self {
            section_info: RegisteredSection::new(channel_name, id, server_id),
            order_user: Vec::new(),
        }
    }

    fn set_user_position(&mut self, user: &str) {
        if let Some(id) = self
            .order_user
            .iter()
            .position(|v| v.eq_ignore_ascii_case(user))
        {
            self.order_user.remove(id);
        }
        self.order_user.push(user.to_string());
    }

    fn remove_user(&mut self, user: &str) {
        if let Some(id) = self
            .order_user
            .iter()
            .position(|v| v.eq_ignore_ascii_case(user))
        {
            self.order_user.remove(id);
        }
    }
}

//List channel contains all the channels
// global is the anonymous channel where the user has sent a message it will be displayed first
// it can also contain unseen user, when we register a user if no channel found we add it in this zone
pub struct UsersWidget {
    list_sections: Vec<Section>,
    list_section_pool_id: usize,

    list_state: ListStateWidget,
    area: Rect,
    need_redraw: bool,
}

impl UsersWidget {
    pub fn new() -> Self {
        Self {
            list_section_pool_id: 0,
            area: Rect::default(),
            list_sections: Vec::new(),
            list_state: ListStateWidget::new(),
            need_redraw: true,
        }
    }

    fn get_section_index(&self, server_id: Option<usize>, section: &str) -> Option<usize> {
        self.list_sections.iter().position(|c| {
            c.section_info.name.eq_ignore_ascii_case(section)
                && c.section_info.server_id == server_id
        })
    }

    fn add_section_index(&mut self, server_id: Option<usize>, section: String) -> usize {
        if let Some(i) = self.list_sections.iter().position(|c| {
            c.section_info.server_id == server_id
                && c.section_info.name.eq_ignore_ascii_case(&section)
        }) {
            i
        } else {
            self.list_sections.push(Section::new(
                section.to_string(),
                self.list_section_pool_id,
                server_id,
            ));
            self.list_section_pool_id = self.list_section_pool_id.saturating_add(1);

            self.list_sections.len().saturating_sub(1)
        }
    }

    fn nb_sections(&self) -> usize {
        self.list_sections.len()
    }

    fn nb_items(&self, section_index: usize) -> usize {
        if let Some(section) = self.list_sections.get(section_index) {
            section.order_user.len() + 1
        } else {
            1
        }
    }

    fn set_users(&mut self, server_id: Option<usize>, section: &str, list_users: Vec<String>) {
        let section_index = self.add_section_index(server_id, section.to_string());

        for user in list_users {
            self.add_user(section_index, &user);
        }
    }

    fn replace_user(
        &mut self,
        irc_model: &crate::irc_view::irc_model::IrcServerModel,
        old: &str,
        new: &str,
    ) {
        let old = Self::sanitize_name(old);
        let new = Self::sanitize_name(new);
        let global_section_id = self
            .get_global_section(Some(irc_model.get_server_id()))
            .map(|v| v.section_info.id);
        if let Some(global_section_id) = global_section_id {
            //not really performant, but not expecting a lot of changes
            for section in self.list_sections.iter_mut() {
                if irc_model.has_user_joined_channel(old, &section.section_info.name)
                    || section.section_info.id == global_section_id
                {
                    for user in section.order_user.iter_mut() {
                        if user.eq_ignore_ascii_case(old) {
                            *user = new.to_string();
                            break;
                        }
                    }
                }
            }
        }
    }

    fn remove_all_users_section(&mut self, server_id: Option<usize>, section: &str) {
        if let Some(section_index) = self.get_section_index(server_id, section)
            && let Some(section) = self.list_sections.get_mut(section_index)
        {
            section.order_user.clear();
        }
    }

    fn remove_user_section(&mut self, server_id: Option<usize>, section: &str, user: &str) {
        if let Some(section_index) = self.get_section_index(server_id, section)
            && let Some(section) = self.list_sections.get_mut(section_index)
        {
            let user = Self::sanitize_name(user);
            section.remove_user(user);
        }
    }

    fn remove_user_all_joined_channels(
        &mut self,
        irc_model: &crate::irc_view::irc_model::IrcServerModel,
        user: &str,
    ) {
        for section in self.list_sections.iter_mut() {
            if irc_model.has_user_joined_channel(user, &section.section_info.name) {
                section.remove_user(user);
            }
        }
    }

    fn highlight_user(&mut self, server_id: Option<usize>, user: &str) {
        //Already selected
        if let Some(selected_name) = self.get_selected_name()
            && selected_name.eq_ignore_ascii_case(user)
        {
            return;
        }

        if let Some(index) = self.get_section_index(server_id, user)
            && let Some(section) = self.list_sections.get_mut(index)
        {
            section.section_info.highlight = true;
        }
    }

    pub fn get_selected_name(&self) -> Option<&str> {
        let (selected, id) = self.list_state.selected();

        if let Some(section) = self.list_sections.get(selected)
            && id > 0
            && let Some(user_name) = section.order_user.get(id.saturating_sub(1))
        {
            Some(user_name)
        } else if let Some(section) = self.list_sections.get(selected) {
            Some(&section.section_info.name)
        } else {
            None
        }
    }

    fn sanitize_name(user: &str) -> &str {
        user.strip_prefix('@').unwrap_or(user)
    }

    fn add_user_with_section(&mut self, server_id: Option<usize>, section: &str, user: &str) {
        if let Some(section_index) = self.get_section_index(server_id, section) {
            self.add_user(section_index, user);
        } else {
            let section_index = self.add_section_index(server_id, section.to_string());
            self.add_user(section_index, user);
        }
    }

    fn add_user(&mut self, section_index: usize, user: &str) {
        let user = UsersWidget::sanitize_name(user);
        if let Some(section) = self.list_sections.get_mut(section_index) {
            section.set_user_position(user);
        }
    }

    //The global section is the first right id
    fn get_global_section(&self, server_id: Option<usize>) -> Option<&Section> {
        self.list_sections
            .iter()
            .find(|v| v.section_info.server_id == server_id)
    }

    fn get_global_section_index(&self, server_id: Option<usize>) -> Option<usize> {
        self.list_sections
            .iter()
            .position(|v| v.section_info.server_id == server_id)
    }

    fn add_user_global_section(&mut self, server_id: Option<usize>, user: &str) {
        if user.starts_with("#") {
            return;
        }

        if let Some(global_section_index) = self.get_global_section_index(server_id) {
            self.add_user(global_section_index, user);
        }
    }

    fn update_selected(
        &mut self,
        previous_selected: (usize, usize),
    ) -> Option<(Option<usize>, String)> {
        let (selected, id) = self.list_state.selected();
        if (previous_selected.0 != selected) || (previous_selected.1 != id) {
            if id > 0
                && let Some(channel) = self.list_sections.get(selected)
                && let Some(user_name) = channel.order_user.get(id - 1)
            {
                self.need_redraw = true;
                Some((channel.section_info.server_id, user_name.to_string()))
            } else if let Some(channel) = self.list_sections.get_mut(selected) {
                channel.section_info.highlight = false;
                self.need_redraw = true;
                Some((
                    channel.section_info.server_id,
                    channel.section_info.name.clone(),
                ))
            } else {
                None
            }
        } else {
            None
        }
    }
}

impl Draw for UsersWidget {
    fn render(
        &mut self,
        irc_model: Option<&crate::irc_view::irc_model::IrcModel>,
        frame: &mut ratatui::Frame<'_>,
        area: ratatui::prelude::Rect,
    ) {
        if self.need_redraw {
            self.need_redraw = false;
        }
        self.area = area;
        if let Some(irc_model) = irc_model {
            self.list_state
                .render(irc_model, &self.list_sections, frame, area);
        }
    }
}
//#channel
// user1
// user2
//#channel2
// user3
// user2
#[derive(Clone, Debug)]
pub struct ListStateWidget {
    current_section: usize,  //#global, #chan
    current_selected: usize, //0 is the main channel, users starts at 1
}

impl ListStateWidget {
    fn new() -> Self {
        Self {
            current_section: 0,
            current_selected: 0,
        }
    }

    fn selected(&self) -> (usize, usize) {
        (self.current_section, self.current_selected)
    }

    fn next(&mut self, max_section: usize) {
        self.current_selected = self.current_selected.saturating_add(1) % max_section;
    }

    fn previous(&mut self, max_section: usize) {
        if self.current_selected == 0 {
            self.current_selected = max_section;
        }
        self.current_selected = self.current_selected.saturating_sub(1) % max_section;
    }

    fn next_section(&mut self, max_nb_sections: usize) {
        self.current_section = self.current_section.saturating_add(1) % max_nb_sections;
        self.current_selected = 0;
    }

    fn previous_section(&mut self, max_nb_sections: usize) {
        if self.current_section == 0 {
            self.current_section = max_nb_sections;
        }
        self.current_section = self.current_section.saturating_sub(1) % max_nb_sections;
        self.current_selected = 0;
    }

    fn add_item<'a>(
        &'a self,
        depth: usize,
        color: ratatui::style::Color,
        title: &'a str,
        is_highlighted: bool,
        is_selected: bool,
    ) -> Vec<ratatui::text::Span<'a>> {
        let mut spans = Vec::new();
        let mut style = Style::default().fg(color);

        if is_selected {
            style = style
                .bg(ratatui::style::Color::Rgb(25, 25, 114))
                .fg(ratatui::style::Color::Rgb(147, 135, 147));
        }

        if is_highlighted {
            style = style.bg(Color::LightBlue);
        }
        spans.push(Span::raw(format!("{:<width$}", " ", width = depth + 1)));
        spans.push(Span::styled(title, style));
        spans
        //ListItem::from(Line::from(spans))
    }

    fn render(
        &mut self,
        irc_model: &crate::irc_view::irc_model::IrcModel,
        sections: &[Section],
        frame: &mut ratatui::Frame<'_>,
        area: ratatui::prelude::Rect,
    ) {
        let mut items = Vec::new();

        for (section_i, section) in sections.iter().enumerate() {
            let item = ListItem::from(Line::from(self.add_item(
                0,
                section.section_info.color,
                &section.section_info.name,
                section.section_info.highlight,
                (self.current_section == section_i) && (self.current_selected == 0),
            )));
            items.push(item);
            if let Some(server_id) = section.section_info.server_id
                && let Some(irc_server) = irc_model.get_server(server_id)
            {
                for (i, user_name) in section.order_user.iter().enumerate() {
                    let spans = self.add_item(
                        1,
                        nickname_color(user_name),
                        user_name,
                        irc_server.has_unread_message(user_name),
                        (self.current_section == section_i) && (self.current_selected == (i + 1)),
                    );

                    let item = ListItem::from(Line::from(spans));
                    items.push(item);
                }
            }
        }
        let list = List::new(items);
        frame.render_widget(list, area);
    }
}

use crate::message_event::MessageEvent;
impl crate::component::EventHandler for UsersWidget {
    fn get_area(&self) -> Rect {
        self.area
    }
    fn need_redraw(&self) -> bool {
        self.need_redraw
    }
    fn handle_actions(
        &mut self,
        irc_model: Option<&crate::irc_view::irc_model::IrcModel>,
        event: &MessageEvent,
    ) -> Option<MessageEvent> {
        match event {
            MessageEvent::UpdateUsers(server_id, channel, list_users) => {
                self.set_users(Some(*server_id), channel, list_users.to_vec());
                self.need_redraw = true;

                None
            }
            MessageEvent::ReplaceUser(server_id, old, new) => {
                if let Some(irc_model) = irc_model
                    && let Some(irc_server) = irc_model.get_server(*server_id)
                {
                    self.replace_user(irc_server, old, new);
                    self.need_redraw = true;
                }
                None
            }
            MessageEvent::Quit(server_id, user, _reason) => {
                if let Some(irc_model) = irc_model
                    && let Some(irc_server) = irc_model.get_server(*server_id)
                {
                    self.remove_user_all_joined_channels(irc_server, user);
                    self.need_redraw = true;
                }
                None
            }
            MessageEvent::PrivMsg(server_id, source, target, _)
            | MessageEvent::ActionMsg(server_id, source, target, _) => {
                if let Some(irc_model) = irc_model
                    && let Some(irc_server) = irc_model.get_server(*server_id)
                {
                    let target = irc_server.get_target(source, target);

                    self.add_user_global_section(Some(*server_id), target);
                    self.highlight_user(Some(*server_id), target);
                    self.need_redraw = true;
                }
                None
            }
            MessageEvent::Part(server_id, channel, user) => {
                if let Some(irc_model) = irc_model
                    && let Some(irc_server) = irc_model.get_server(*server_id)
                {
                    if irc_server.is_main_user(user) {
                        self.remove_all_users_section(Some(*server_id), channel);
                    } else {
                        self.remove_user_section(Some(*server_id), channel, user);
                    }
                    self.need_redraw = true;
                }

                None
            }
            MessageEvent::JoinServer(server_id, server) => {
                self.add_section_index(Some(*server_id), server.to_string());
                self.need_redraw = true;
                None
            }
            MessageEvent::Join(server_id, channel, user) => {
                let section_index = self.add_section_index(Some(*server_id), channel.to_string());
                self.add_user_with_section(Some(*server_id), channel, user);
                self.list_state.current_section = section_index;
                self.need_redraw = true;

                None
            }
            MessageEvent::SelectChannel(server_id, channel) => {
                if let Some(index) = self.get_section_index(*server_id, channel) {
                    self.list_state.current_section = index;
                }
                self.need_redraw = true;
                None
            }
            _ => None,
        }
    }

    fn handle_events(
        &mut self,
        event: &crate::event_handler::Event,
    ) -> Option<crate::message_event::MessageEvent> {
        if let Some(key) = event.get_key()
            && key.is_press()
            && !key.is_repeat()
        {
            let previous = self.list_state.clone();
            let previous_selected = previous.selected();
            let number_items = self.nb_items(self.list_state.current_section);
            let number_sections = self.nb_sections();

            if key.modifiers.contains(KeyModifiers::CONTROL) && number_items > 0 {
                match key.code {
                    crossterm::event::KeyCode::Char('p') => {
                        self.list_state.previous(number_items);
                    }
                    crossterm::event::KeyCode::Char('n') => {
                        self.list_state.next(number_items);
                    }
                    _ => {}
                }
            } else if key.modifiers.contains(KeyModifiers::ALT) && number_sections > 0 {
                match key.code {
                    crossterm::event::KeyCode::Up => {
                        self.list_state.previous_section(number_sections);
                    }
                    crossterm::event::KeyCode::Down => {
                        self.list_state.next_section(number_sections);
                    }
                    _ => {}
                }
            }

            return self
                .update_selected(previous_selected)
                .map(|(a, b)| MessageEvent::SelectChannel(a, b));
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{component::EventHandler, irc_view::irc_model::IrcModel};

    const TEST_SERVER_ID: usize = 0;

    struct WidgetTest {
        pub users_widget: UsersWidget,
        pub irc_model: Option<IrcModel>,
    }

    impl WidgetTest {
        fn handle_action(&mut self, action: &MessageEvent) {
            self.users_widget
                .handle_actions(self.irc_model.as_ref(), action);
            if let Some(irc_model) = self.irc_model.as_mut() {
                irc_model.handle_action(action);
            }
        }

        fn join_server(&mut self, server_name: &str) {
            self.handle_action(&MessageEvent::JoinServer(
                TEST_SERVER_ID,
                server_name.to_string(),
            ));
        }

        fn join_channel(&mut self, channel: &str, user: &str) {
            self.handle_action(&MessageEvent::Join(
                TEST_SERVER_ID,
                channel.to_string(),
                user.to_string(),
            ));
        }

        fn join_channel_users(&mut self, channel: &str, users: Vec<&str>) {
            let action = MessageEvent::UpdateUsers(
                TEST_SERVER_ID,
                channel.to_string(),
                users
                    .into_iter()
                    .map(|v| v.to_string())
                    .collect::<Vec<String>>(),
            );
            self.handle_action(&action);
        }

        fn next_section(&mut self) -> Option<String> {
            let previous_selected = self.users_widget.list_state.selected();
            let number_sections = self.users_widget.nb_sections();
            self.users_widget.list_state.next_section(number_sections);

            let v = self.users_widget.update_selected(previous_selected);
            if let Some((server_id, channel_name)) = &v {
                self.handle_action(&MessageEvent::SelectChannel(
                    *server_id,
                    channel_name.clone(),
                ));
            }
            v.map(|(_, name)| name)
        }

        fn next_item(&mut self) -> Option<String> {
            let previous_selected = self.users_widget.list_state.selected();
            let number_items = self
                .users_widget
                .nb_items(self.users_widget.list_state.current_section);
            self.users_widget.list_state.next(number_items);

            let v = self.users_widget.update_selected(previous_selected);
            if let Some((server_id, name)) = &v {
                self.handle_action(&MessageEvent::SelectChannel(*server_id, name.clone()));
            }
            v.map(|(_, name)| name)
        }
    }

    #[test]
    fn test_join_speak_quit_channel() {
        let users_widget = UsersWidget::new();
        let user_name = "farine";
        let channel = "#rust";
        let server_name = "IRC-Server";
        let irc_model = Some(crate::irc_view::irc_model::IrcModel::new_single_server(
            1,
            TEST_SERVER_ID,
            user_name.to_string(),
        ));

        let mut widget_test = WidgetTest {
            irc_model,
            users_widget,
        };

        widget_test.join_server(server_name);
        assert_eq!(widget_test.users_widget.nb_sections(), 1);

        widget_test.join_channel(channel, user_name);
        assert_eq!(widget_test.users_widget.nb_sections(), 2);
        assert_eq!(
            widget_test.users_widget.list_sections[0].section_info.name,
            server_name
        );
        assert_eq!(
            &widget_test.users_widget.list_sections[1].section_info.name,
            channel
        );
        assert_eq!(
            &widget_test.users_widget.list_sections[1].order_user[0],
            user_name
        );

        widget_test.join_channel_users(channel, vec!["a", "b"]);
        assert_eq!(
            widget_test.users_widget.list_sections[1].order_user.len(),
            3
        );

        // a to farine (Private Message)
        let action = MessageEvent::PrivMsg(
            TEST_SERVER_ID,
            "a".to_string(),
            user_name.to_string(),
            "Message".to_string(),
        );
        widget_test.handle_action(&action);

        // Global section (index 0) should now have user "a"
        assert_eq!(
            widget_test.users_widget.list_sections[0].order_user.len(),
            1
        );

        // 'a' quits
        let action = MessageEvent::Quit(TEST_SERVER_ID, "a".to_string(), None);
        widget_test.handle_action(&action);

        // Global section keeps user (per your remove_user_from_all_except_global logic)
        assert_eq!(
            widget_test.users_widget.list_sections[0].order_user.len(),
            1
        );
        // Channel section removes user
        assert_eq!(
            widget_test.users_widget.list_sections[1].order_user.len(),
            2
        );
    }

    #[test]
    fn test_join_speak_then_select() {
        let users_widget = UsersWidget::new();
        let user_name = "farine";
        let channel = "#rust";
        let server_name = "IRC-Server";
        let mut irc_model = crate::irc_view::irc_model::IrcModel::new(1);
        irc_model.init_server(0, user_name.to_string());

        let mut widget_test = WidgetTest {
            irc_model: Some(irc_model),
            users_widget,
        };

        widget_test.join_server(server_name);
        widget_test.join_channel(channel, user_name);
        widget_test.join_channel_users(channel, vec!["a"]);

        let action = MessageEvent::PrivMsg(
            TEST_SERVER_ID,
            "a".to_string(),
            user_name.to_string(),
            "Message".to_string(),
        );
        widget_test.handle_action(&action);

        assert_eq!(
            widget_test.users_widget.list_sections[0].order_user.len(),
            1
        );
        assert_eq!(widget_test.users_widget.list_state.current_section, 1);

        assert_eq!(widget_test.next_item(), Some("farine".to_string()));
        assert_eq!(widget_test.next_item(), Some("a".to_string()));
        assert_eq!(widget_test.next_item(), Some("#rust".to_string()));
    }

    #[test]
    fn test_join_speak_part_channel() {
        let users_widget = UsersWidget::new();
        let user_name = "farine";
        let channel = "#rust";
        let server_name = "IRC-Server";
        let irc_model = Some(crate::irc_view::irc_model::IrcModel::new_single_server(
            1,
            TEST_SERVER_ID,
            user_name.to_string(),
        ));

        let mut widget_test = WidgetTest {
            irc_model,
            users_widget,
        };

        let action = MessageEvent::JoinServer(TEST_SERVER_ID, server_name.to_string());
        widget_test.handle_action(&action);

        let action = MessageEvent::Join(TEST_SERVER_ID, channel.to_string(), user_name.to_string());
        widget_test.handle_action(&action);

        let action = MessageEvent::UpdateUsers(
            TEST_SERVER_ID,
            channel.to_string(),
            vec!["a", "b"].into_iter().map(|v| v.to_string()).collect(),
        );
        widget_test.handle_action(&action);

        let action = MessageEvent::ReplaceUser(TEST_SERVER_ID, "a".to_string(), "c".to_string());
        widget_test.handle_action(&action);

        assert_eq!(
            widget_test.users_widget.list_sections[1]
                .order_user
                .contains(&"c".to_string()),
            true
        );

        let action = MessageEvent::Part(TEST_SERVER_ID, channel.to_string(), "c".to_string());
        widget_test.handle_action(&action);

        assert_eq!(
            widget_test.users_widget.list_sections[1].order_user.len(),
            2
        );
    }

    #[test]
    fn test_add_section_uppercase() {
        let mut users_widget = UsersWidget::new();
        let section = "#rust";
        users_widget.add_section_index(Some(TEST_SERVER_ID), section.to_string());
        users_widget.add_section_index(Some(TEST_SERVER_ID), section.to_uppercase());

        assert_eq!(users_widget.list_sections.len(), 1);
        assert_eq!(users_widget.list_sections[0].section_info.name, "#rust");
    }
}
