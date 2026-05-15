use super::completion::Completion;
use super::history::InputHistory;
use super::text_input::InputWidget;
#[cfg(feature = "spell-checker")]
use {super::spell_checker::SpellChecker, crate::message_irc::message_content::MessageKind};

use crate::component::Draw;
use crate::message_event::MessageEvent;
use crossterm::event::KeyCode;
use ratatui::{
    Frame,
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::Paragraph,
};

#[derive(Default)]
pub struct CInput {
    input: InputWidget,
    input_history: InputHistory,
    completion: Completion,
    area: Rect,
    redraw: bool,

    #[cfg(feature = "spell-checker")]
    spell_checker: Option<SpellChecker>,
    #[cfg(feature = "spell-checker")]
    spellchecker_task: Option<crate::async_task::AsyncTask<SpellChecker>>,
}

impl Draw for CInput {
    fn render(&mut self, _ctx: &mut crate::state::context::Ctx, frame: &mut Frame<'_>, area: Rect) {
        self.area = area;
        if self.redraw {
            self.redraw = false;
        }
        // keep 2 for borders and 1 for cursor
        let width = area.width.max(3) - 3;
        let scroll = self.input.compute_visual_scroll(width as usize);
        #[cfg(feature = "spell-checker")]
        let input = if self.spell_checker.is_some() {
            let mut spans = Vec::new();
            spans.push(Span::from("> ").style(Style::default().fg(ratatui::style::Color::Cyan)));
            spans.extend(self.spans_with_spellcheck(self.input.get_value()));
            Paragraph::new(Line::from(spans))
        } else {
            self.default_paragraph()
        };

        #[cfg(not(feature = "spell-checker"))]
        let input = self.default_paragraph();

        frame.render_widget(input.scroll((0, scroll as u16)), area);

        let x = self.input.visual_cursor().max(scroll) - scroll + 2;
        frame.set_cursor_position((area.x + x as u16, area.y))
    }
}

impl crate::component::EventHandler for CInput {
    fn get_area(&self) -> Rect {
        self.area
    }
    fn need_redraw(&self) -> bool {
        self.redraw
    }
    fn handle_actions(
        &mut self,
        ctx: &mut crate::state::context::Ctx,
        event: &MessageEvent,
    ) -> Option<MessageEvent> {
        match event {
            #[cfg(feature = "spell-checker")]
            MessageEvent::SpellChecker(language) => {
                if let Some(language) = language {
                    self.spellchecker_task = Some(crate::async_task::AsyncTask {
                        handle: Some(SpellChecker::async_build(language)),
                        result: None,
                    });
                } else {
                    self.spell_checker = None;
                    self.spellchecker_task = None;
                }
                None
            }
            MessageEvent::UpdateUsers(server_id, channel, users) => {
                self.completion
                    .input_completion
                    .add_users(*server_id, channel, users);
                None
            }
            MessageEvent::ReplaceUser(_, old, new) => {
                self.completion.input_completion.replace_user(old, new);
                None
            }
            MessageEvent::SelectChannel(server_id, channel) => {
                self.completion.current_channel = channel.to_string();
                self.completion.server_id = *server_id;

                None
            }
            MessageEvent::SettingsDidChange => {
                let (empty, middle) = ctx.model.get_completion_behaviour();
                self.completion.set_completion_behaviour(
                    empty.map(|v| v.to_string()).unwrap_or_default(),
                    middle.map(|v| v.to_string()).unwrap_or_default(),
                );

                None
            }
            MessageEvent::Join(server_id, channel, user) => {
                self.completion.current_channel = channel.to_string();
                self.completion
                    .input_completion
                    .add_user(*server_id, channel, user.to_string());

                None
            }
            MessageEvent::Part(server_id, channel, user) => {
                if ctx.session.model.is_main_user(*server_id, user) {
                    self.completion
                        .input_completion
                        .remove_channel(*server_id, channel);
                } else {
                    self.completion
                        .input_completion
                        .disable_user(*server_id, channel, user);
                }

                None
            }
            _ => None,
        }
    }

    fn handle_events(
        &mut self,
        _ctx: &mut crate::state::context::Ctx,
        event: &crate::event_handler::Event,
    ) -> Option<MessageEvent> {
        match event {
            crate::event_handler::Event::Crossterm(event) => {
                if let Some(key_event) = event.as_key_event() {
                    self.redraw = true;
                    match key_event.code {
                        KeyCode::Enter => {
                            let m = self.get_current_input().to_string();
                            self.reset_value();
                            if !m.is_empty() {
                                self.input_history.add_message(m.clone());
                                Some(MessageEvent::MessageInput(m))
                            } else {
                                None
                            }
                        }
                        KeyCode::Down => {
                            if key_event.modifiers.is_empty() {
                                self.input_history.down();
                                if let Some(m) = self.input_history.get_message() {
                                    self.input.reset_with(m.to_string());
                                } else {
                                    self.input.reset();
                                }
                            }

                            None
                        }
                        KeyCode::Up => {
                            if key_event.modifiers.is_empty() {
                                self.input_history.up(&self.input.value);
                                if let Some(m) = self.input_history.get_message() {
                                    self.input.reset_with(m.to_string());
                                }
                            }
                            None
                        }
                        KeyCode::Tab => {
                            let middle = self.input.find_previous_break(false).unwrap_or(0) > 0;
                            self.set_completion();
                            if let Some((index, value)) =
                                self.completion.get_next_completion(middle)
                            {
                                self.input.insert_completion(index, value);
                            }
                            None
                        }
                        _ => {
                            self.completion.reset();
                            self.input.handle_key_events(&key_event);
                            None
                        }
                    }
                } else if let crossterm::event::Event::Paste(content) = event {
                    self.redraw = true;
                    self.input.handle_paste(content.to_string());
                    None
                } else {
                    None
                }
            }
            #[cfg(feature = "spell-checker")]
            crate::event_handler::Event::Tick => self.handle_spellchecker(),
            _ => None,
        }
    }
}

impl CInput {
    fn set_completion(&mut self) {
        if let Some(start) = self.input.find_previous_break(false).or(Some(0)) {
            self.completion.set_completion(
                start,
                self.input.cursor_position,
                self.input.get_value(),
            );
        }
    }

    fn default_paragraph(&self) -> Paragraph<'_> {
        Paragraph::new(Line::from(vec![
            Span::from("> ").style(Style::default().fg(ratatui::style::Color::Cyan)),
            Span::from(self.input.get_value()),
        ]))
    }

    #[cfg(feature = "spell-checker")]
    pub fn spans_with_spellcheck<'a>(&self, input: &'a str) -> Vec<Span<'a>> {
        let mut spans = Vec::new();
        let mut start = 0;
        let mut in_word = false;
        if let Some(spell_checker) = self.spell_checker.as_ref() {
            for (i, ch) in input.char_indices() {
                if ch.is_ascii_whitespace() {
                    if in_word {
                        let word = &input[start..i];

                        let color = if !spell_checker.check_word(word) {
                            ratatui::style::Color::LightBlue
                        } else {
                            ratatui::style::Color::default()
                        };
                        spans.push(Span::from(word).style(Style::default().bg(color)));
                        in_word = false;
                    }

                    spans.push(Span::from(&input[i..i + ch.len_utf8()]));
                    start = i + ch.len_utf8();
                } else if !in_word {
                    in_word = true;
                    start = i;
                }
            }
            if in_word {
                let word = &input[start..];
                let color = if !spell_checker.check_word(&word.to_lowercase()) {
                    ratatui::style::Color::LightBlue
                } else {
                    ratatui::style::Color::default()
                };
                spans.push(Span::from(word).style(Style::default().bg(color)));
            }
        }

        spans
    }

    pub fn get_current_input(&self) -> &str {
        self.input.get_value()
    }

    pub fn reset_value(&mut self) {
        self.input.reset();
    }

    #[cfg(feature = "spell-checker")]
    fn handle_spellchecker(&mut self) -> Option<MessageEvent> {
        if self.spellchecker_task.as_mut().is_some_and(|v| v.poll())
            && let Some(spell_task) = self.spellchecker_task.take()
        {
            if let Some(spell_checker) = spell_task.take_result() {
                match spell_checker {
                    Ok(spell_checker) => {
                        self.spell_checker = Some(spell_checker);
                        Some(MessageEvent::AddMessageViewInfo(
                            None,
                            None,
                            MessageKind::Error,
                            "Spell checker is ready".to_string(),
                        ))
                    }
                    Err(e) => Some(MessageEvent::AddMessageViewInfo(
                        None,
                        None,
                        MessageKind::Error,
                        format!("Spell checker error: {}", e),
                    )),
                }
            } else {
                Some(MessageEvent::AddMessageViewInfo(
                    None,
                    None,
                    MessageKind::Error,
                    "Error no spell checker retrieved".to_string(),
                ))
            }
        } else {
            None
        }
    }

    pub fn add_completion_command_list(&mut self, values: impl Iterator<Item = String>) {
        for c in values {
            self.completion.input_completion.add_command(c);
        }
    }

    pub fn set_completion_config_list(&mut self, values: impl Iterator<Item = String>) {
        self.completion.input_completion.clear_config();
        for c in values {
            self.completion.input_completion.add_config_field(c);
        }
    }
}
