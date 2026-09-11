//! Full-frame render benchmark.
//!
//! The history sizes below exist to make that scaling visible, and to give
//! samply/perf a deterministic, loop-shaped workload for the real render path:
//!
//!     RUSTFLAGS="-C force-frame-pointers=yes" \
//!       cargo bench -p clown --features bench --profile profiling --bench render_frame

use clown::component::Draw;
use clown::irc_view::discuss::discuss_widget::DiscussWidget;
use clown::irc_view::discuss::servers_messages::ServersMessages;
use clown::message_irc::message_content::MessageContent;
use clown::state::context::Ctx;
use clown::state::model::Model;
use clown::state::server_id::ServerID;
use clown::state::session::Session;
use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use ratatui::{Terminal, backend::TestBackend};
use std::hint::black_box;

const SERVER_ID: ServerID = ServerID::new(0);
const CHANNEL: &str = "#rust";

/// Deterministic corpus generator, so two profiles are comparable.
struct Lcg(u64);

impl Lcg {
    fn next_u64(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        self.0 >> 33
    }

    fn pick<'a, T>(&mut self, items: &'a [T]) -> Option<&'a T> {
        let index = (self.next_u64() as usize) % items.len().max(1);
        items.get(index)
    }
}

const NICKS: &[&str] = &[
    "alice",
    "bob",
    "carol",
    "dave",
    "erin",
    "frank_",
    "grace",
    "heidi",
    "ivan",
    "judy",
    "mallory99",
    "niaj",
    "olivia",
    "peggy",
    "rupert",
];

/// Roughly the shape of a real channel: mostly short lines, a long tail that
/// wraps over several rows, and the occasional non-ASCII message.
const BODIES: &[&str] = &[
    "yeah",
    "that works, thanks",
    "hmm",
    "did you try cargo clean first?",
    "https://github.com/F4r3n/clown/pull/42",
    "I think the borrow checker is right there, the lifetime outlives the loop",
    "Lorem Ipsum is simply dummy text of the printing and typesetting industry. \
     Lorem Ipsum has been the industry's standard dummy text ever since the 1500s, \
     when an unknown printer took a galley of type and scrambled it to make a type \
     specimen book",
    "ok",
    "the profiler says we spend 60% of the frame in wrapped_line_count which is wild",
    "héllo wörld — em dash and accents to exercise the unicode width path",
    "+1",
    "rebased and force-pushed, should be green now",
];

fn next_message(rng: &mut Lcg) -> MessageContent {
    // ~1 in 8 messages is a join/part/quit style info line.
    if rng.next_u64() % 8 == 0 {
        let nick = rng.pick(NICKS).copied().unwrap_or("alice");
        MessageContent::info(format!("{nick} has joined {CHANNEL}"))
    } else {
        let nick = rng.pick(NICKS).copied().unwrap_or("alice");
        let body = rng.pick(BODIES).copied().unwrap_or("yeah");
        MessageContent::message(Some(nick.to_string()), body.to_string())
    }
}

fn build_ctx(message_count: usize) -> (Ctx, DiscussWidget) {
    let mut messages = ServersMessages::new(std::path::PathBuf::new());
    let mut widget = DiscussWidget::new();

    widget.add_server_group(
        &mut messages,
        Some(SERVER_ID),
        Some("irc.libera.chat".to_string()),
    );
    widget.set_current_channel(Some(SERVER_ID), CHANNEL);

    let mut rng = Lcg(0x5eed);
    for _ in 0..message_count {
        widget.add_line(
            &mut messages,
            Some(SERVER_ID),
            CHANNEL,
            next_message(&mut rng),
        );
    }

    let ctx = Ctx {
        session: Session::new(1),
        model: Model::new_empty_config(),
        messages,
    };

    (ctx, widget)
}

fn draw_once(terminal: &mut Terminal<TestBackend>, ctx: &mut Ctx, widget: &mut DiscussWidget) {
    // TestBackend writes into an in-memory buffer, so this never fails; the
    // result is dropped rather than unwrapped to stay inside the workspace lints.
    let _ = terminal.draw(|frame| {
        let area = frame.area();
        widget.render(ctx, frame, area);
    });
}

/// How the frame cost grows with channel history, at a fixed terminal size.
/// A flat line here would mean render is O(visible rows); it is not.
fn bench_history_size(c: &mut Criterion) {
    let mut group = c.benchmark_group("render_frame/history");

    for &count in &[100usize, 1_000, 10_000, 50_000] {
        let (mut ctx, mut widget) = build_ctx(count);
        let mut terminal = match Terminal::new(TestBackend::new(120, 40)) {
            Ok(terminal) => terminal,
            Err(_) => continue,
        };

        group.throughput(Throughput::Elements(count as u64));
        group.bench_with_input(BenchmarkId::from_parameter(count), &count, |b, _| {
            b.iter(|| {
                draw_once(&mut terminal, black_box(&mut ctx), black_box(&mut widget));
            });
        });
    }

    group.finish();
}

/// The scrollbar sizing walk alone: `get_fake_total_lines` re-measures every
/// message in the channel on every frame, so its cost is O(history) no matter
/// where the viewport sits or how few rows are actually visible.
fn bench_total_lines(c: &mut Criterion) {
    let mut group = c.benchmark_group("render_frame/total_lines");

    for &count in &[100usize, 1_000, 10_000, 50_000] {
        let (mut ctx, mut widget) = build_ctx(count);
        let mut terminal = match Terminal::new(TestBackend::new(120, 40)) {
            Ok(terminal) => terminal,
            Err(_) => continue,
        };
        // One real render first: `content_width` is only set by the `Layout`
        // inside `render`, and the walk is a no-op while it is still zero.
        draw_once(&mut terminal, &mut ctx, &mut widget);

        group.throughput(Throughput::Elements(count as u64));
        group.bench_with_input(BenchmarkId::from_parameter(count), &count, |b, _| {
            b.iter(|| black_box(&widget).total_lines(black_box(&ctx.messages)));
        });
    }

    group.finish();
}

/// Same history, viewport moved progressively further back. `find_viewport_start`
/// walks from the newest message towards the target, so cost should climb with
/// scroll depth even though the number of drawn rows is constant.
fn bench_scroll_depth(c: &mut Criterion) {
    let mut group = c.benchmark_group("render_frame/scroll_depth");
    let (mut ctx, mut widget) = build_ctx(10_000);

    for &offset in &[0usize, 1_000, 5_000, 9_000] {
        let mut terminal = match Terminal::new(TestBackend::new(120, 40)) {
            Ok(terminal) => terminal,
            Err(_) => continue,
        };
        widget.set_scroll_offset(offset);

        group.bench_with_input(BenchmarkId::from_parameter(offset), &offset, |b, _| {
            b.iter(|| {
                draw_once(&mut terminal, black_box(&mut ctx), black_box(&mut widget));
            });
        });
    }

    group.finish();
}

/// Terminal geometry at a fixed history: width drives the wrap work per message,
/// height drives how many rows actually get built.
fn bench_geometry(c: &mut Criterion) {
    let mut group = c.benchmark_group("render_frame/geometry");
    let (mut ctx, mut widget) = build_ctx(10_000);

    for &(width, height) in &[(80u16, 24u16), (120, 40), (200, 50), (400, 100)] {
        let mut terminal = match Terminal::new(TestBackend::new(width, height)) {
            Ok(terminal) => terminal,
            Err(_) => continue,
        };

        group.bench_function(
            BenchmarkId::from_parameter(format!("{width}x{height}")),
            |b| {
                b.iter(|| {
                    draw_once(&mut terminal, black_box(&mut ctx), black_box(&mut widget));
                });
            },
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_history_size,
    bench_total_lines,
    bench_scroll_depth,
    bench_geometry
);
criterion_main!(benches);
