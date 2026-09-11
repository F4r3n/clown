//! IRC formatting primitives, benched over a corpus of input shapes.
//!
//! `is_string_plain` is the gate every other entry point here opens with: if a
//! message has no control bytes, `to_spans`, `get_width_without_format` and
//! `strip_irc_formatting_cow` all take a cheap path that borrows instead of
//! rebuilding. So the interesting axis is not message length but *where* the
//! first control byte sits — hence the `control/leading` and `control/trailing`
//! cases, which bracket the fast path's best and worst behaviour.
//!
//! Throughput is reported in bytes, so ns/byte is comparable across shapes:
//!
//!     cargo bench -p clown --bench message_parser
//!     cargo bench -p clown --bench message_parser -- to_spans   # one group

use clown::message_irc::message_parser::{
    get_width_without_format, is_string_plain, strip_irc_formatting_cow, to_spans,
};
use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use std::hint::black_box;

const LOREM: &str = "Lorem Ipsum is simply dummy text of the printing and typesetting industry. \
     Lorem Ipsum has been the industry's standard dummy text ever since the 1500s, when an \
     unknown printer took a galley of type and scrambled it to make a type specimen book";

struct Input {
    name: &'static str,
    text: String,
}

impl Input {
    fn new(name: &'static str, text: impl Into<String>) -> Self {
        Self {
            name,
            text: text.into(),
        }
    }
}

/// Wraps every word in its own colour code: the worst case for `to_spans`,
/// which pushes a span per style change.
fn dense_colors(text: &str) -> String {
    let mut out = String::with_capacity(text.len() * 2);
    for (i, word) in text.split_whitespace().enumerate() {
        out.push_str(&format!("\x03{},{}", i % 16, (i + 3) % 16));
        out.push_str(word);
        out.push(' ');
    }
    out
}

fn corpus() -> Vec<Input> {
    vec![
        // The shape that actually dominates a live channel.
        Input::new("plain/short", "that works, thanks"),
        Input::new("plain/long", LOREM),
        // Non-ASCII: `str::width()` can no longer stay on its ASCII fast path.
        Input::new(
            "plain/unicode",
            "héllo wörld — em dash and accents to exercise the unicode width path",
        ),
        // Formatted, but mostly literal text between the codes.
        Input::new(
            "color/sparse",
            "Lorem Ipsum is simply \x038,4dummy\x0f text of the printing and typesetting \
             industry. Lorem Ipsum has been the \x034standard\x0f dummy text ever since 1500",
        ),
        Input::new("color/dense", dense_colors(LOREM)),
        // Bold toggles only: the cheaper control bytes, no colour digit scanning.
        Input::new(
            "modifier/bold",
            "\x02rebased\x02 and \x02force-pushed\x02, should be \x02green\x02 now",
        ),
        // Bails out of `is_string_plain` on the second byte...
        Input::new("control/leading", format!("\x02{LOREM}")),
        // ...versus scanning the whole string only to then take the slow path
        // anyway. The gap between these two is the cost of the fast-path probe.
        Input::new("control/trailing", format!("{LOREM}\x0f")),
    ]
}

/// Runs `f` over every input as one criterion group.
///
/// The closures below return a scalar derived from the result rather than the
/// result itself: `to_spans` and `strip_irc_formatting_cow` borrow from their
/// argument, which a fixed return type cannot express. The upside is that the
/// `Vec`/`Cow` is dropped inside the timed region, so the allocation *and* its
/// free are counted — which is what callers actually pay.
fn bench_group<O>(
    c: &mut Criterion,
    name: &str,
    inputs: &[Input],
    mut f: impl FnMut(&str) -> O,
) {
    let mut group = c.benchmark_group(name);

    for input in inputs {
        group.throughput(Throughput::Bytes(input.text.len() as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(input.name),
            input.text.as_str(),
            |b, text| {
                b.iter(|| f(black_box(text)));
            },
        );
    }

    group.finish();
}

fn criterion_benchmark(c: &mut Criterion) {
    let inputs = corpus();

    bench_group(c, "message_parser/is_string_plain", &inputs, |text| {
        is_string_plain(text)
    });

    bench_group(c, "message_parser/to_spans", &inputs, |text| {
        to_spans(text, None).len()
    });

    bench_group(c, "message_parser/width_without_format", &inputs, |text| {
        get_width_without_format(text)
    });

    bench_group(c, "message_parser/strip_formatting", &inputs, |text| {
        strip_irc_formatting_cow(text).len()
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
