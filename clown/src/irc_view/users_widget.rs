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
    color: ratatui::style::Color,
    highlight: bool,
}

impl RegisteredSection {
    pub fn new(name: String, id: usize) -> Self {
        Self {
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
    fn new(channel_name: String, id: usize) -> Self {
        Self {
            section_info: RegisteredSection::new(channel_name, id),
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

    fn get_section_id(&self, section: &str) -> Option<usize> {
        self.list_sections
            .iter()
            .find(|c| c.section_info.name.eq_ignore_ascii_case(section))
            .map(|c| c.section_info.id)
    }

    fn add_section(&mut self, section: String) -> Option<usize> {
        let index = if let Some(i) = self
            .list_sections
            .iter()
            .position(|c| c.section_info.name.eq_ignore_ascii_case(&section))
        {
            i
        } else {
            self.list_sections
                .push(Section::new(section.to_string(), self.list_section_pool_id));
            self.list_section_pool_id = self.list_section_pool_id.saturating_add(1);

            self.list_sections.len().saturating_sub(1)
        };
        self.list_sections.get(index).map(|c| c.section_info.id)
    }

    fn nb_sections(&self) -> usize {
        self.list_sections.len()
    }

    fn nb_items(&self, section_id: usize) -> usize {
        if let Some(section) = self.list_sections.get(section_id) {
            section.order_user.len() + 1
        } else {
            1
        }
    }

    fn set_users(&mut self, section: &str, list_users: Vec<String>) {
        let section_id = self.add_section(section.to_string());

        if let Some(section) = section_id {
            for user in list_users {
                self.add_user(section, &user);
            }
        }
    }

    fn replace_user(
        &mut self,
        irc_model: &crate::irc_view::irc_model::IrcModel,
        old: &str,
        new: &str,
    ) {
        let old = Self::sanitize_name(old);
        let new = Self::sanitize_name(new);
        let global_section_id = self.get_global_section_id();

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

    fn remove_all_users_section(&mut self, section_id: Option<usize>) {
        if let Some(id) = section_id
            && let Some(section) = self.list_sections.get_mut(id)
        {
            section.order_user.clear();
        }
    }

    fn remove_user_from_all_except_global(
        &mut self,
        irc_model: &crate::irc_view::irc_model::IrcModel,
        user: &str,
    ) {
        for section in self.list_sections.iter_mut() {
            if irc_model.has_user_joined_channel(user, &section.section_info.name) {
                section.remove_user(user);
            }
        }
    }

    fn remove_user(&mut self, section_id: usize, user: &str) {
        let user = UsersWidget::sanitize_name(user);

        if let Some(section) = self.list_sections.get_mut(section_id) {
            section.remove_user(user);
        }
    }

    fn hightlight_user(&mut self, user: &str) {
        //Already selected
        if let Some(selected_name) = self.get_selected_name()
            && selected_name.eq_ignore_ascii_case(user)
        {
            return;
        }

        if let Some(id) = self.get_section_id(user)
            && let Some(section) = self.list_sections.get_mut(id)
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

    fn add_user_with_section(&mut self, section: &str, user: &str) {
        if let Some(section_id) = self.get_section_id(section) {
            self.add_user(section_id, user);
        } else if let Some(section_id) = self.add_section(section.to_string()) {
            self.add_user(section_id, user);
        }
    }

    fn add_user(&mut self, section_id: usize, user: &str) {
        let user = UsersWidget::sanitize_name(user).to_string();
        if let Some(section) = self.list_sections.get_mut(section_id) {
            section.set_user_position(&user);
        }
    }

    fn get_global_section_id(&self) -> usize {
        0
    }

    fn add_user_global_section(&mut self, user: &str) {
        if let Some(global_section) = self.list_sections.get(self.get_global_section_id())
            && global_section.section_info.name.eq(user)
        {
            return;
        }

        if !user.starts_with("#") {
            self.add_user(self.get_global_section_id(), user);
        }
    }

    fn update_selected(&mut self, previous_selected: (usize, usize)) -> Option<String> {
        let (selected, id) = self.list_state.selected();
        if (previous_selected.0 != selected) || (previous_selected.1 != id) {
            if id > 0
                && let Some(channel) = self.list_sections.get(selected)
                && let Some(user_name) = channel.order_user.get(id - 1)
            {
                self.need_redraw = true;
                Some(user_name.to_string())
            } else if let Some(channel) = self.list_sections.get_mut(selected) {
                channel.section_info.highlight = false;
                self.need_redraw = true;
                Some(channel.section_info.name.clone())
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
        irc_model: &crate::irc_view::irc_model::IrcModel,
        frame: &mut ratatui::Frame<'_>,
        area: ratatui::prelude::Rect,
    ) {
        if self.need_redraw {
            self.need_redraw = false;
        }
        self.area = area;
        self.list_state
            .render(irc_model, &self.list_sections, frame, area);
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
            current_section: 1,
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

            for (i, user_name) in section.order_user.iter().enumerate() {
                let spans = self.add_item(
                    1,
                    nickname_color(user_name),
                    user_name,
                    irc_model.has_unread_message(user_name),
                    (self.current_section == section_i) && (self.current_selected == (i + 1)),
                );

                let item = ListItem::from(Line::from(spans));
                items.push(item);
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
        irc_model: &crate::irc_view::irc_model::IrcModel,
        event: &MessageEvent,
    ) -> Option<MessageEvent> {
        match event {
            MessageEvent::UpdateUsers(channel, list_users) => {
                self.set_users(channel, list_users.to_vec());
                self.need_redraw = true;

                None
            }
            MessageEvent::ReplaceUser(old, new) => {
                self.replace_user(irc_model, old, new);
                self.need_redraw = true;

                None
            }
            MessageEvent::Quit(user, _reason) => {
                self.remove_user_from_all_except_global(irc_model, user);
                self.need_redraw = true;
                None
            }
            MessageEvent::PrivMsg(source, target, _)
            | MessageEvent::ActionMsg(source, target, _) => {
                let target = irc_model.get_target(source, target);

                self.add_user_global_section(target);
                self.hightlight_user(target);
                self.need_redraw = true;

                None
            }
            MessageEvent::Part(channel, user) => {
                if irc_model.is_main_user(user) {
                    self.remove_all_users_section(self.get_section_id(channel));
                } else if let Some(channel_id) = self.get_section_id(channel) {
                    self.remove_user(channel_id, user);
                }
                self.need_redraw = true;

                None
            }
            MessageEvent::JoinServer(server) => {
                self.add_section(server.to_string());
                self.need_redraw = true;
                None
            }
            MessageEvent::Join(channel, user) => {
                self.add_section(channel.to_string());
                self.add_user_with_section(channel, user);

                if let Some(id) = self.get_section_id(channel) {
                    self.list_state.current_section = id;
                }
                self.need_redraw = true;

                None
            }
            MessageEvent::SelectChannel(channel) => {
                if let Some(id) = self.get_section_id(channel) {
                    self.list_state.current_section = id;
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
                .map(MessageEvent::SelectChannel);
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use crate::{component::EventHandler, irc_view::irc_model::IrcModel};

    use super::*;

    struct WidgetTest {
        pub users_widget: UsersWidget,
        pub irc_model: IrcModel,
    }

    impl WidgetTest {
        fn handle_action(&mut self, action: &MessageEvent) {
            self.users_widget.handle_actions(&self.irc_model, action);
            self.irc_model.handle_action(action);
        }

        fn join_server(&mut self, server_name: &str) {
            self.handle_action(&MessageEvent::JoinServer(server_name.to_string()));
        }

        fn join_channel(&mut self, channel: &str, user: &str) {
            self.handle_action(&MessageEvent::Join(channel.to_string(), user.to_string()));
        }

        fn join_channel_users(&mut self, channel: &str, users: Vec<&str>) {
            let action = MessageEvent::UpdateUsers(
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
            if let Some(v) = &v {
                self.handle_action(&MessageEvent::SelectChannel(v.to_string()));
            }
            v
        }

        fn previous_section(&mut self) -> Option<String> {
            let previous_selected = self.users_widget.list_state.selected();

            let number_sections = self.users_widget.nb_sections();
            self.users_widget
                .list_state
                .previous_section(number_sections);

            let v = self.users_widget.update_selected(previous_selected);
            if let Some(v) = &v {
                self.handle_action(&MessageEvent::SelectChannel(v.to_string()));
            }
            v
        }

        fn next_item(&mut self) -> Option<String> {
            let previous_selected = self.users_widget.list_state.selected();
            let number_items = self
                .users_widget
                .nb_items(self.users_widget.list_state.current_section);
            self.users_widget.list_state.next(number_items);

            let v = self.users_widget.update_selected(previous_selected);
            if let Some(v) = &v {
                self.handle_action(&MessageEvent::SelectChannel(v.to_string()));
            }
            v
        }

        fn previous_item(&mut self) -> Option<String> {
            let previous_selected = self.users_widget.list_state.selected();
            let number_items = self
                .users_widget
                .nb_items(self.users_widget.list_state.current_section);
            self.users_widget.list_state.previous(number_items);

            let v = self.users_widget.update_selected(previous_selected);
            if let Some(v) = &v {
                self.handle_action(&MessageEvent::SelectChannel(v.to_string()));
            }
            v
        }
    }

    #[test]
    fn test_join_speak_quit_channel() {
        let users_widget = UsersWidget::new();
        let user_name = "farine";
        let channel = "#rust";
        let server_name = "IRC-Server";
        let irc_model = crate::irc_view::irc_model::IrcModel::new_model(
            user_name.to_string(),
            channel.to_string(),
        );
        let mut widget_test = WidgetTest {
            irc_model,
            users_widget,
        };
        //Join server
        widget_test.join_server(server_name);
        assert_eq!(widget_test.users_widget.nb_sections(), 1);

        //join channel
        widget_test.join_channel(channel, user_name);
        assert_eq!(widget_test.users_widget.nb_sections(), 2);
        assert_eq!(
            widget_test.users_widget.list_sections[0].section_info.name,
            server_name
        );
        assert_eq!(
            &widget_test.users_widget.list_sections[1].section_info.name,
            &channel
        );
        assert_eq!(
            &widget_test.users_widget.list_sections[1].order_user[0],
            &user_name
        );

        // a and b join channel
        widget_test.join_channel_users(channel, vec!["a", "b"]);

        assert_eq!(widget_test.users_widget.nb_sections(), 2);
        assert_eq!(
            widget_test.users_widget.list_sections[1].order_user.len(),
            3
        );
        assert_eq!(
            widget_test.users_widget.list_sections[0].order_user.len(),
            0
        );

        //me to 'a'
        let action = MessageEvent::PrivMsg(
            user_name.to_string(),
            "a".to_string(),
            "Message".to_string(),
        );
        widget_test.handle_action(&action);

        assert_eq!(
            widget_test.users_widget.list_sections[0].order_user.len(),
            1
        );

        //'a' quits
        let action = MessageEvent::Quit(user_name.to_string(), None);
        widget_test.handle_action(&action);

        assert_eq!(
            widget_test.users_widget.list_sections[0].order_user.len(),
            1
        );
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
        let irc_model = crate::irc_view::irc_model::IrcModel::new_model(
            user_name.to_string(),
            channel.to_string(),
        );
        let mut widget_test = WidgetTest {
            irc_model,
            users_widget,
        };
        widget_test.join_server(server_name);
        widget_test.join_channel(channel, user_name);

        // a join channel
        widget_test.join_channel_users(channel, vec!["a"]);

        //a to farine
        let action = MessageEvent::PrivMsg(
            "a".to_string(),
            user_name.to_string(),
            "Message".to_string(),
        );
        widget_test.handle_action(&action);

        assert_eq!(
            widget_test.users_widget.list_sections[0].order_user.len(),
            1
        );
        assert!(widget_test.irc_model.has_unread_message("a"));
        assert_eq!(widget_test.users_widget.list_state.current_section, 1);

        assert_eq!(widget_test.next_item(), Some("farine".to_string()));
        assert!(widget_test.irc_model.has_unread_message("a"));

        assert_eq!(widget_test.next_item(), Some("a".to_string()));
        assert!(!widget_test.irc_model.has_unread_message("a"));

        assert_eq!(widget_test.next_item(), Some("#rust".to_string()));
    }

    #[test]
    fn test_join_speak_part_channel() {
        let mut users_widget = UsersWidget::new();
        let user_name = "farine";
        let channel = "#rust";
        let server_name = "IRC-Server";
        let mut irc_model = crate::irc_view::irc_model::IrcModel::new_model(
            user_name.to_string(),
            channel.to_string(),
        );

        //Join server
        let action = MessageEvent::JoinServer(server_name.to_string());

        users_widget.handle_actions(&irc_model, &action);
        irc_model.handle_action(&action);

        assert_eq!(users_widget.nb_sections(), 1);

        //join channel
        let action = MessageEvent::Join(channel.to_string(), user_name.to_string());
        users_widget.handle_actions(&irc_model, &action);
        irc_model.handle_action(&action);

        // a and b join channel
        let action = MessageEvent::UpdateUsers(
            channel.to_string(),
            vec!["a", "b"]
                .into_iter()
                .map(|v| v.to_string())
                .collect::<Vec<String>>(),
        );

        users_widget.handle_actions(&irc_model, &action);
        irc_model.handle_action(&action);

        //me to 'a'
        let action = MessageEvent::PrivMsg(
            user_name.to_string(),
            "a".to_string(),
            "Message".to_string(),
        );
        users_widget.handle_actions(&irc_model, &action);
        irc_model.handle_action(&action);

        let action = MessageEvent::ReplaceUser("a".to_string(), "c".to_string());
        users_widget.handle_actions(&irc_model, &action);
        irc_model.handle_action(&action);

        assert_eq!(users_widget.list_sections[0].order_user.len(), 1);
        assert_eq!(users_widget.list_sections[0].order_user[0], "c".to_string());
        assert_eq!(irc_model.get_user("a"), None);
        assert_eq!(irc_model.get_user("c").unwrap().get_name(), "c");

        //'a' part
        let action = MessageEvent::Part(channel.to_string(), "c".to_string());
        users_widget.handle_actions(&irc_model, &action);
        irc_model.handle_action(&action);

        assert_eq!(users_widget.list_sections[0].order_user.len(), 1);
        assert_eq!(users_widget.list_sections[1].order_user.len(), 2);
    }

    #[test]
    fn test_join_speak_rename() {
        let users_widget = UsersWidget::new();
        let user_name = "farine";
        let channel = "#rust";
        let server_name = "IRC-Server";
        let irc_model = crate::irc_view::irc_model::IrcModel::new_model(
            user_name.to_string(),
            channel.to_string(),
        );
        let mut widget_test = WidgetTest {
            irc_model,
            users_widget,
        };
        widget_test.join_server(server_name);
        widget_test.join_channel(channel, user_name);

        // a join channel
        widget_test.join_channel_users(channel, vec!["a"]);

        //a to farine
        let action = MessageEvent::PrivMsg(
            "a".to_string(),
            user_name.to_string(),
            "Message".to_string(),
        );
        widget_test.handle_action(&action);

        //nick 'a' to 'c'
        let action = MessageEvent::ReplaceUser("a".to_string(), "c".to_string());
        widget_test.handle_action(&action);
        assert_eq!(
            widget_test.users_widget.list_sections[0].order_user.len(),
            1
        );
        assert_eq!(
            widget_test.users_widget.list_sections[0].order_user[0],
            "c".to_string()
        );

        assert_eq!(
            widget_test.users_widget.list_sections[1].order_user.len(),
            2
        );
        assert_eq!(
            widget_test.users_widget.list_sections[1].order_user[1],
            "c".to_string()
        );
    }

    #[test]
    fn test_add_section_uppercase() {
        let mut users_widget = UsersWidget::new();
        let section = "#rust";
        users_widget.add_section(section.to_string());
        users_widget.add_section(section.to_uppercase());
        assert_eq!(users_widget.list_sections.len(), 1);
        assert_eq!(users_widget.list_sections[0].section_info.name, "#rust");

        let section = "#rUst2";
        users_widget.add_section(section.to_string());
        users_widget.add_section(section.to_uppercase());
        assert_eq!(users_widget.list_sections.len(), 2);
    }
}
