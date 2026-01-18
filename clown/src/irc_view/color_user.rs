use palette::FromColor;
use palette::{Oklch, Srgb};
use ratatui::style::Color;

fn hash_nickname(nickname: &str) -> u64 {
    let state = ahash::RandomState::with_seeds(1, 2, 3, 4);
    state.hash_one(nickname)
}

pub fn nickname_color(nickname: &str) -> ratatui::style::Color {
    let hash = hash_nickname(nickname);
    bright_distinct_color(hash)
}

fn compute_color(index: u64) -> Oklch<f32> {
    let hue_index: u32 = (index >> 32) as u32;
    let lower = index as u32;
    let lum_index: u16 = (lower >> 16) as u16;
    let chroma_index: u16 = lower as u16;
    // Spread hue evenly around the color wheel
    let hue = (hue_index % 360) as f32;

    let lumf = (f32::from(lum_index)) / (f32::from(u16::MAX));
    let chromaf = (f32::from(chroma_index)) / (f32::from(u16::MAX));

    let luminance = lumf.max(0.15).powf(0.5);
    let chroma = 0.11 + (chromaf * 0.1);
    Oklch::new(luminance, chroma, hue)
}

pub fn bright_distinct_color(index: u64) -> Color {
    let lch = compute_color(index);

    let srgb: Srgb<u8> = Srgb::from_color(lch).into_format();
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
                brightness > 40.0,
                "Color for '{}' ({r},{g},{b}) too dark (brightness={brightness})",
                name
            );
        }
    }

    #[test]
    fn test_color_brightness_extended() {
        let nicknames = vec![
            "A",
            "Alice",
            "bob",
            "LongNicknameWithManyCharacters",
            "123456",
            "!@#$%^",
            " ",
            "🦀",
            "Very Long Name with Spaces",
            "admin",
            "root",
            "guest",
            "z",
        ];

        for name in nicknames {
            let color = nickname_color(name);
            let (r, g, b) = color_to_tuple(color);

            // Rec. 601 luma formula
            let brightness = 0.299 * r as f32 + 0.587 * g as f32 + 0.114 * b as f32;

            assert!(
                brightness > 40.0,
                "Color for '{}' (RGB: {},{},{}) is too dark. Brightness: {:.2}",
                name,
                r,
                g,
                b,
                brightness
            );
        }
    }

    #[test]
    fn test_color_distribution_histogram() {
        let num_samples = 10_000;
        let num_buckets = 12; // 30 degrees per bucket (12 * 30 = 360)
        let mut histogram = vec![0; num_buckets];

        for i in 0..num_samples {
            // Simulate unique names
            let name = format!("user_{}", i);
            let oklch = compute_color(hash_nickname(&name));
            let h = oklch.hue.into_positive_degrees();

            let bucket_index = (h / (360.0 / num_buckets as f32)) as usize;
            histogram[bucket_index.min(num_buckets - 1)] += 1;
        }

        // Check for "Clustering"
        // In a perfect world, each bucket has 10,000 / 12 = ~833 hits.
        let expected_avg = num_samples / num_buckets;
        let tolerance = expected_avg / 2;

        for (i, count) in histogram.iter().enumerate() {
            let range_start = i * (360 / num_buckets);
            let range_end = (i + 1) * (360 / num_buckets);

            assert!(
                *count > (expected_avg - tolerance),
                "Hue gap detected at {}°–{}°! Only {} colors generated.",
                range_start,
                range_end,
                count
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
