use crate::{component::Draw, irc_view::color_user::nickname_color};
use ahash::AHashMap;
use bit_vec::BitVec;
use crossterm::event::KeyModifiers;
use ratatui::{
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{List, ListItem},
};
#[derive(Debug, PartialEq)]
struct User {
    name: String,
    need_hightlight: bool,
    color: ratatui::style::Color,
    connected_sections: BitVec,
}
const NB_SECTIONS: usize = 32;

impl User {
    pub fn new(name: String) -> Self {
        Self {
            need_hightlight: false,
            color: nickname_color(&name),
            name,
            connected_sections: BitVec::from_elem(NB_SECTIONS, false),
        }
    }

    pub fn set_name(&mut self, name: String) {
        self.name = name;
        self.color = nickname_color(&self.name);
    }

    pub fn join_section(&mut self, id: usize) {
        if let Some(mut channel) = self.connected_sections.get_mut(id) {
            *channel = true;
        }
    }

    pub fn has_joined_section(&mut self, id: usize) -> bool {
        self.connected_sections.get(id).unwrap_or(false)
    }

    pub fn quit_section(&mut self, id: usize) {
        if let Some(mut channel) = self.connected_sections.get_mut(id) {
            *channel = false;
        }
    }

    pub fn quit_all_except_global(&mut self) {
        let mut mask = BitVec::from_elem(NB_SECTIONS, false);
        mask.set(0, true);
        self.connected_sections.and(&mask);
    }

    pub fn has_joined_any_section(&mut self) -> bool {
        self.connected_sections.any()
    }
}

impl DrawableItem for User {
    fn display_color(&self) -> ratatui::style::Color {
        self.color
    }

    fn display_title(&self) -> &str {
        &self.name
    }

    fn is_highlighted(&self) -> bool {
        self.need_hightlight
    }
}

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

trait DrawableItem {
    fn display_title(&self) -> &str;
    fn display_color(&self) -> ratatui::style::Color;
    fn is_highlighted(&self) -> bool;
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
// list users contain to which channel a user belong. The ID is based on the index of the list_sections
pub struct UsersWidget {
    list_sections: Vec<Section>,
    list_section_pool_id: usize,
    list_users: ahash::AHashMap<String, User>,

    list_state: ListStateWidget,
    area: Rect,
    need_redraw: bool,
}

impl UsersWidget {
    pub fn new() -> Self {
        Self {
            list_users: AHashMap::new(),
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

    fn replace_user(&mut self, old: &str, new: &str) {
        let old = UsersWidget::sanitize_name(old);
        let new = UsersWidget::sanitize_name(new);

        let v = self.list_users.remove(old);

        if let Some(mut v) = v {
            let n = new;
            v.set_name(n.to_string());

            //not really performant, but not expecting a lot of changes
            for section in self.list_sections.iter_mut() {
                if v.has_joined_section(section.section_info.id) {
                    for user in section.order_user.iter_mut() {
                        if user.eq_ignore_ascii_case(old) {
                            *user = new.to_string();
                            break;
                        }
                    }
                }
            }
            self.list_users.insert(n.to_string(), v);
        }
    }

    fn remove_all_users_section(&mut self, section_id: Option<usize>) {
        if let Some(id) = section_id
            && let Some(section) = self.list_sections.get_mut(id)
        {
            for user_name in section.order_user.iter() {
                if let Some(user) = self.list_users.get_mut(user_name) {
                    user.quit_section(id);
                }
            }
            section.order_user.clear();
        }
    }

    fn remove_user_from_all(&mut self, user: &str) {
        if let Some(u) = self.list_users.get_mut(user) {
            u.quit_all_except_global();
            if !u.has_joined_any_section() {
                self.list_users.remove(user);
            }
        }
    }

    fn remove_user(&mut self, section_id: usize, user: &str) {
        let user = UsersWidget::sanitize_name(user);

        if let Some(u) = self.list_users.get_mut(user) {
            if let Some(section) = self.list_sections.get_mut(section_id) {
                section.remove_user(user);
            }

            u.quit_section(section_id);
        }
    }

    fn hightlight_user(&mut self, user: &str) {
        //Already selected
        if let Some(selected_name) = self.get_selected_name()
            && selected_name.eq_ignore_ascii_case(user)
        {
            return;
        }

        if let Some(user) = self.list_users.get_mut(user) {
            user.need_hightlight = true;
        } else if let Some(id) = self.get_section_id(user)
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
        if let Some(user) = self.list_users.get_mut(&user) {
            user.join_section(section_id);
        } else {
            let mut new_user = User::new(user.to_string());
            new_user.join_section(section_id);
            self.list_users.insert(user, new_user);
        }
    }

    fn get_global_section_id(&self) -> usize {
        0
    }

    fn add_user_global_section(&mut self, user: &str) {
        if !user.starts_with("#") {
            self.add_user(self.get_global_section_id(), user);
        }
    }
}

impl Draw for UsersWidget {
    fn render(&mut self, frame: &mut ratatui::Frame<'_>, area: ratatui::prelude::Rect) {
        if self.need_redraw {
            self.need_redraw = false;
        }
        self.area = area;
        self.list_state
            .render(&self.list_sections, &self.list_users, frame, area);
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
            style = style.bg(ratatui::style::Color::Rgb(25, 25, 114));
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
        sections: &[Section],
        users: &AHashMap<String, User>,
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
                if let Some(user) = users.get(user_name) {
                    let spans = self.add_item(
                        1,
                        user.display_color(),
                        user.display_title(),
                        user.is_highlighted(),
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
    fn handle_actions(&mut self, event: &MessageEvent) -> Option<MessageEvent> {
        match event {
            MessageEvent::UpdateUsers(channel, list_users) => {
                self.set_users(channel, list_users.to_vec());
                self.need_redraw = true;

                None
            }
            MessageEvent::ReplaceUser(old, new) => {
                self.replace_user(old, new);
                self.need_redraw = true;

                None
            }
            MessageEvent::Quit(user, _) => {
                self.remove_user_from_all(user);
                self.need_redraw = true;

                None
            }
            MessageEvent::Part(channel, user, is_main) => {
                if *is_main {
                    self.remove_all_users_section(self.get_section_id(channel));
                } else if let Some(channel_id) = self.get_section_id(channel) {
                    self.remove_user(channel_id, user);
                }
                self.need_redraw = true;

                None
            }
            MessageEvent::HighlightUser(user) => {
                self.add_user_global_section(user);
                self.hightlight_user(user);
                self.need_redraw = true;

                None
            }
            MessageEvent::Join(channel, user, _main) => {
                self.add_section(channel.to_string());
                if let Some(user) = user {
                    self.add_user_with_section(channel, user);
                }
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

            let (selected, id) = self.list_state.selected();
            if (previous_selected.0 != selected) || (previous_selected.1 != id) {
                if id > 0
                    && let Some(channel) = self.list_sections.get(selected)
                    && let Some(user_name) = channel.order_user.get(id - 1)
                    && let Some(user) = self.list_users.get_mut(user_name)
                {
                    user.need_hightlight = false;
                    self.need_redraw = true;
                    return Some(MessageEvent::SelectChannel(user.name.to_string()));
                } else if let Some(channel) = self.list_sections.get_mut(selected) {
                    channel.section_info.highlight = false;
                    self.need_redraw = true;
                    return Some(MessageEvent::SelectChannel(
                        channel.section_info.name.clone(),
                    ));
                }
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_user() {
        let mut users_widget = UsersWidget::new();
        let user_name = "farine";
        users_widget.add_user_with_section("#spam", user_name);
        assert_eq!(users_widget.list_sections.len(), 1);
        assert!(users_widget.list_users.get(user_name).is_some());
        let user = users_widget.list_users.get(user_name).unwrap();
        assert_eq!(user.name, user_name.to_string());
        assert_eq!(user.color, nickname_color(user_name));
        assert_eq!(users_widget.list_users.len(), 1);
    }

    #[test]
    fn test_add_user_multiple_section() {
        let mut users_widget = UsersWidget::new();
        let user_name = "farine";
        users_widget.add_user_with_section("#spam", user_name);
        users_widget.add_user_with_section("#spam_2", user_name);
        assert_eq!(users_widget.list_sections.len(), 2);
        assert_eq!(users_widget.list_users.len(), 1);
        assert_eq!(
            users_widget.list_sections.get(0).unwrap().order_user.len(),
            1
        );
        assert_eq!(
            users_widget.list_sections.get(1).unwrap().order_user.len(),
            1
        );

        users_widget.add_user_with_section("#spam_2", "@farine");
        assert_eq!(users_widget.list_users.len(), 1);
    }

    #[test]
    fn test_replace_user_multiple_section() {
        let mut users_widget = UsersWidget::new();
        let user_name = "farine";
        users_widget.add_user_with_section("#spam", user_name);
        users_widget.add_user_with_section("#spam_2", user_name);

        users_widget.replace_user("farine", "chuck");
        assert_eq!(
            users_widget
                .list_users
                .get("chuck")
                .map(|v| v.name.to_string()),
            Some("chuck".to_string())
        );

        assert_eq!(
            users_widget.list_sections[0].order_user.first(),
            Some(&"chuck".to_string())
        );
    }

    #[test]
    fn test_add_gloabl_section() {
        let mut users_widget = UsersWidget::new();
        let user_name = "IRC-Server";
        users_widget.add_user_with_section("", user_name);
        assert_eq!(users_widget.list_sections.len(), 1);
        assert_eq!(
            users_widget.list_sections.get(0).unwrap().order_user.len(),
            1
        );

        assert_eq!(users_widget.list_users.len(), 1);
    }

    #[test]
    fn test_add_gloabl_user_section() {
        let mut users_widget = UsersWidget::new();
        let user_name = "user1";
        users_widget.add_user_with_section("IRC-Server", user_name);
        assert_eq!(users_widget.list_sections.len(), 1);
        assert_eq!(
            users_widget.list_sections.get(0).unwrap().order_user.len(),
            1
        );

        assert_eq!(users_widget.list_users.len(), 1);

        users_widget.add_user_global_section("TEST"); //When a user is added in the global it cannot be removed
        assert_eq!(users_widget.list_users.len(), 2);

        users_widget.remove_all_users_section(users_widget.get_section_id("TEST"));
        assert_eq!(users_widget.list_users.len(), 2);
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

    #[test]
    fn test_number_sections() {
        let mut users_widget = UsersWidget::new();
        let user_name = "IRC-Server";
        users_widget.add_user_with_section(user_name, user_name);
        assert_eq!(users_widget.nb_sections(), 1);

        users_widget.add_user_with_section("#new-chan", user_name);
        assert_eq!(users_widget.nb_sections(), 2);

        assert_eq!(users_widget.nb_items(0), 2);
        assert_eq!(users_widget.nb_items(1), 2);
    }
}
