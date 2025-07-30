use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

fn hash_nickname(nickname: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    nickname.hash(&mut hasher);
    hasher.finish()
}

pub fn nickname_color(nickname: &str) -> ratatui::style::Color {
    let hash = hash_nickname(nickname);
    let r = hash & 0xFF;
    let g = (hash >> 8) & 0xFF;
    let b = (hash >> 16) & 0xFF;
    ratatui::style::Color::Rgb(r as u8, g as u8, b as u8)
}
