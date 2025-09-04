use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::ops::Add;

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
    let mut hsl = hsl::HSL::from_rgb(&[r as u8, g as u8, b as u8]);

    //Lighter
    if hsl.l < 0.5 {
        hsl.l = hsl.l.add(0.2_f64).clamp(0.0, 1.0);
    }
    let (r, g, b) = hsl.to_rgb();
    ratatui::style::Color::Rgb(r, g, b)
}
