use palette::convert::FromColorUnclamped;
use palette::{Hsv, Srgb, encoding::Srgb as EncSrgb};
use ratatui::style::Color;

fn hash_nickname(nickname: &str) -> u64 {
    let state = ahash::RandomState::with_seeds(1, 2, 3, 4);
    state.hash_one(nickname)
}

pub fn nickname_color(nickname: &str) -> ratatui::style::Color {
    let hash = hash_nickname(nickname);
    bright_distinct_color(hash)
}

pub fn bright_distinct_color(index: u64) -> Color {
    let hue_index: u32 = (index >> 32) as u32;
    let lower = index as u32;
    let sat_index: u16 = (lower >> 16) as u16;
    let val_index: u16 = lower as u16;
    // Spread hue evenly around the color wheel
    let hue = (hue_index % 360) as f32;

    let satf = (f32::from(sat_index)) / (f32::from(u16::MAX));
    let valf = (f32::from(val_index)) / (f32::from(u16::MAX));

    //println!("{} {}", index_upper, index_lower);
    let saturation = 0.2 + 0.8 * satf; // 0.55–0.9
    let value = 0.5 + 0.5 * valf; // 0.8–1
    let hsv: Hsv<EncSrgb, f32> = Hsv::new(hue, saturation, value);

    let srgb: Srgb<u8> = Srgb::from_color_unclamped(hsv).into_format();
    Color::Rgb(srgb.red, srgb.green, srgb.blue)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::style::Color;

    fn color_to_tuple(color: Color) -> (u8, u8, u8) {
        match color {
            Color::Rgb(r, g, b) => (r, g, b),
            _ => panic!("Expected Color::Rgb"),
        }
    }

    #[test]
    fn deterministic_for_same_name() {
        let a1 = nickname_color("Alice");
        let a2 = nickname_color("Alice");
        assert_eq!(
            a1, a2,
            "Color should be deterministic for the same nickname"
        );
    }

    #[test]
    fn different_names_produce_different_colors() {
        let a = nickname_color("Alice");
        let b = nickname_color("Bob");
        let c = nickname_color("Charlie");
        assert_ne!(a, b, "Different nicknames should give different colors");
        assert_ne!(a, c, "Different nicknames should give different colors");
        assert_ne!(b, c, "Different nicknames should give different colors");
    }

    #[test]
    fn color_channels_are_in_valid_range() {
        let color = nickname_color("Test");
        let (r, g, b) = color_to_tuple(color);
        for (name, v) in [("r", r), ("g", g), ("b", b)] {
            assert!(
                (0..=255).contains(&v),
                "Channel {} out of range ({} not in 0..=255)",
                name,
                v
            );
        }
    }

    #[test]
    fn color_is_bright_for_dark_background() {
        // Test a few nicknames and ensure brightness (luminance) is decent.
        for i in 0..26 {
            let c = char::from(b'a' + i);
            let mut name = String::new();
            name.push(c);

            let (r, g, b) = color_to_tuple(nickname_color(&name));
            //https://stackoverflow.com/questions/596216/formula-to-determine-perceived-brightness-of-rgb-color
            let brightness = 0.299 * r as f32 + 0.587 * g as f32 + 0.114 * b as f32;
            assert!(
                brightness > 45.0,
                "Color for '{}' ({r},{g},{b}) too dark (brightness={brightness})",
                name
            );
        }
    }

    #[test]
    fn consistent_under_long_names() {
        // Ensure long strings don't overflow or behave weirdly
        let long_name = "x".repeat(1000);
        let color = nickname_color(&long_name);
        let (r, g, b) = color_to_tuple(color);
        assert!(
            (0..=255).contains(&r) && (0..=255).contains(&g) && (0..=255).contains(&b),
            "Color out of range for long nickname"
        );
    }

    #[test]
    fn test_distinct_color() {
        let color_a = nickname_color("guill");
        let color_b = nickname_color("farine");

        assert!(color_a != color_b);
    }
}
