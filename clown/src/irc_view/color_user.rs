use ahash::AHashMap;
use palette::FromColor;
use palette::{Oklch, Srgb};
use ratatui::style::Color;

pub struct ColorGenerator {
    seed: u64,
    overrides: ahash::AHashMap<String, Color>,
}

impl ColorGenerator {
    pub fn new(seed: u64) -> Self {
        Self {
            seed,
            overrides: AHashMap::new(),
        }
    }

    fn parse_hex_color(input: &str) -> Option<Color> {
        if !input.starts_with('#') || input.len() != 7 {
            return None;
        }
        let r = u8::from_str_radix(input.get(1..3)?, 16).ok()?;
        let g = u8::from_str_radix(input.get(3..5)?, 16).ok()?;
        let b = u8::from_str_radix(input.get(5..7)?, 16).ok()?;
        Some(Color::Rgb(r, g, b))
    }

    pub fn add_override(&mut self, input: String, color: &str) -> bool {
        if let Some(c) = Self::parse_hex_color(color) {
            self.overrides.insert(input, c);
            true
        } else {
            false
        }
    }

    pub fn generate_color(&self, input: &str) -> Color {
        self.overrides
            .get(input)
            .cloned()
            .unwrap_or(self.nickname_color(input))
    }

    fn nickname_color(&self, nickname: &str) -> ratatui::style::Color {
        let hash = hash_nickname(self.seed, nickname);
        bright_distinct_color(hash)
    }
}

fn hash_nickname(seed: u64, nickname: &str) -> u64 {
    let state = ahash::RandomState::with_seeds(
        seed.saturating_add(1),
        seed.saturating_add(2),
        seed.saturating_add(3),
        seed.saturating_add(4),
    );
    state.hash_one(nickname)
}

fn compute_color(index: u64) -> Oklch<f32> {
    let lower = index as u32;
    let lum_index: u16 = (lower >> 16) as u16;
    let chroma_index: u16 = lower as u16;
    // Spread hue evenly around the color wheel
    let hue = (index % 360) as f32;

    let lumf = (f32::from(lum_index)) / (f32::from(u16::MAX));
    let chromaf = (f32::from(chroma_index)) / (f32::from(u16::MAX));

    let luminance = lumf.max(0.3).powf(0.7);
    let chroma = 0.02 + (chromaf * 0.25);
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
        let color_generator = ColorGenerator::new(1);
        let a1 = color_generator.generate_color("Alice");
        let a2 = color_generator.generate_color("Alice");
        assert_eq!(
            a1, a2,
            "Color should be deterministic for the same nickname"
        );
    }

    #[test]
    fn different_names_produce_different_colors() {
        let color_generator = ColorGenerator::new(1);

        let a = color_generator.generate_color("Alice");
        let b = color_generator.generate_color("Bob");
        let c = color_generator.generate_color("Charlie");
        assert_ne!(a, b, "Different nicknames should give different colors");
        assert_ne!(a, c, "Different nicknames should give different colors");
        assert_ne!(b, c, "Different nicknames should give different colors");
    }

    #[test]
    fn color_channels_are_in_valid_range() {
        let color_generator = ColorGenerator::new(1);

        let color = color_generator.generate_color("Test");
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
        let color_generator = ColorGenerator::new(1);

        // Test a few nicknames and ensure brightness (luminance) is decent.
        for i in 0..26 {
            let c = char::from(b'a' + i);
            let mut name = String::new();
            name.push(c);

            let (r, g, b) = color_to_tuple(color_generator.generate_color(&name));
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
            let color_generator = ColorGenerator::new(1);

            let color = color_generator.generate_color(name);
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
            let oklch = compute_color(hash_nickname(0, &name));
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
        let color_generator = ColorGenerator::new(1);

        // Ensure long strings don't overflow or behave weirdly
        let long_name = "x".repeat(1000);
        let color = color_generator.generate_color(&long_name);
        let (r, g, b) = color_to_tuple(color);
        assert!(
            (0..=255).contains(&r) && (0..=255).contains(&g) && (0..=255).contains(&b),
            "Color out of range for long nickname"
        );
    }

    #[test]
    fn test_distinct_color() {
        let color_generator = ColorGenerator::new(1);

        let color_a = color_generator.generate_color("guill");
        let color_b = color_generator.generate_color("farine");

        assert!(color_a != color_b);
    }

    fn print_colored_nickname(name: &str) {
        let color_generator = ColorGenerator::new(1);

        let color = color_generator.generate_color(name);

        // We match on the Ratatui color to extract RGB values
        if let ratatui::style::Color::Rgb(r, g, b) = color {
            // \x1b[38;2;R;G;Bm sets the foreground color (text)
            // \x1b[48;2;R;G;Bm sets the background color
            // \x1b[0m resets the formatting
            println!(
                "\x1b[38;2;{r};{g};{b}m█\x1b[0m {name:.<12} -> Rgb({r}, {g}, {b})",
                r = r,
                g = g,
                b = b,
                name = name
            );
        } else {
            println!("{:?}: {}", color, name);
        }
    }

    #[test]
    fn test_distinct_color_() {
        let color_generator = ColorGenerator::new(1);

        let color_a = color_generator.generate_color("guill");
        let color_b = color_generator.generate_color("farine");

        assert!(color_a != color_b);
        for i in 0..26 {
            let c = char::from(b'a' + i);
            let mut name = String::new();
            name.push(c);
            /*name.push(c);
            name.push(c);
            name.push(c);
            name.push(c);*/

            print_colored_nickname(&name);
        }
    }
}
