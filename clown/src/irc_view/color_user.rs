use palette::convert::FromColorUnclamped;
use palette::{Hsv, Lab, Srgb, encoding::Srgb as EncSrgb};
use ratatui::style::Color;

fn hash_nickname(nickname: &str) -> u64 {
    let state = ahash::RandomState::with_seeds(1, 2, 3, 4);

    // Build a new hasher with that state
    state.hash_one(nickname)
}

pub fn nickname_color(nickname: &str) -> ratatui::style::Color {
    let hash = hash_nickname(nickname);
    bright_distinct_color(hash)
}

pub fn bright_distinct_color(index: u64) -> Color {
    const GOLDEN_RATIO_CONJUGATE: f32 = 0.618034;
    const GOLDEN_ANGLE: f32 = 360.0 * GOLDEN_RATIO_CONJUGATE;

    // Spread hue evenly around the color wheel
    let hue = (index as f32 * GOLDEN_ANGLE) % 360.0;

    // Use quasi-random variations for S and V (repeat period is irrational)
    let f = index as f32 * GOLDEN_RATIO_CONJUGATE;
    let saturation = 0.55 + 0.35 * (f.fract() - 0.5).abs(); // 0.55–0.9
    let value = 0.8 + 0.15 * ((f * 1.37).fract() - 0.5).abs(); // 0.8–0.95

    let hsv: Hsv<EncSrgb, f32> = Hsv::new(hue, saturation, value);
    let lab: Lab = Lab::from_color_unclamped(hsv);

    // Lighten perceptually, not with hard clamp
    let lab = Lab {
        l: lab.l * 1.1, // brighten slightly, keep variation
        ..lab
    };

    let srgb: Srgb<u8> = Srgb::from_color_unclamped(lab).into_format();
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
        let names = ["A", "B", "C", "Zed", "omega"];
        for &name in &names {
            let (r, g, b) = color_to_tuple(nickname_color(name));
            let brightness = 0.299 * r as f32 + 0.587 * g as f32 + 0.114 * b as f32;
            assert!(
                brightness > 100.0,
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
}
