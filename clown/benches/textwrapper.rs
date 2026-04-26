use clown::message_irc::textwrapper::{wrap_content, wrap_spans, wrapped_line_count};
use criterion::{Criterion, criterion_group, criterion_main};
use std::hint::black_box;

fn bench_wrap_line_count(message: &str, width: usize) -> anyhow::Result<()> {
    wrapped_line_count(message, width);
    Ok(())
}

fn bench_wrap_text(message: &str, width: usize) -> anyhow::Result<()> {
    wrap_content(message, width);
    Ok(())
}

fn bench_wrap_spans(message: &str, width: usize) -> anyhow::Result<()> {
    wrap_spans(message, width, None);
    Ok(())
}

fn criterion_benchmark(c: &mut Criterion) {
    c.bench_function("wrap_spans", |b| {
        b.iter(|| {
            bench_wrap_spans(
                black_box(
                    "Lorem Ipsum is simply dummy text of the printing and typesetting industry.",
                ),
                black_box(80),
            )
        });
        b.iter(|| {
            bench_wrap_spans(
                black_box(
                    "Lorem Ipsum is simply \x038,4Hi! text of the printing and \x038,4Hi! industry.
            Lorem Ipsum has \x038,4Hi! the industry's standard \x038,4Hi! text ever since the 1500s",
                ),
                black_box(30),
            )
        });

        b.iter(|| {
            bench_wrap_spans(
                black_box(
                    "Lorem Ipsum is simply \x038,4Hi! text of the printing and \x038,4Hi! industry.
            Lorem Ipsum has \x038,4Hi! the industry's standard \x038,4Hi! text ever since the 1500s",
                ),
                black_box(30),
            )
        });
    });

    c.bench_function("Wrap count text", |b| {
        b.iter(|| {
            bench_wrap_line_count(
                black_box(
                    "Lorem Ipsum is simply \x038,4Hi! text of the printing and \x038,4Hi! industry.
            Lorem Ipsum has \x038,4Hi! the industry's standard \x038,4Hi! text ever since the 1500s",
                ),
                black_box(10),
            )
        });

        b.iter(|| {
            bench_wrap_line_count(
                black_box(
                    "Lorem Ipsum is simply \x038,4Hi! text of the printing and \x038,4Hi! industry.
            Lorem Ipsum has \x038,4Hi! the industry's standard \x038,4Hi! text ever since the 1500s",
                ),
                black_box(100),
            )
        });
    });

    c.bench_function("Wrap text", |b| {
        b.iter(|| {
            bench_wrap_text(
                black_box(
                    "Lorem Ipsum is simply \x038,4Hi! text of the printing and \x038,4Hi! industry.
            Lorem Ipsum has \x038,4Hi! the industry's standard \x038,4Hi! text ever since the 1500s",
                ),
                black_box(10),
            )
        });

        b.iter(|| {
            bench_wrap_text(
                black_box(
                    "Lorem Ipsum is simply \x038,4Hi! text of the printing and \x038,4Hi! industry.
            Lorem Ipsum has \x038,4Hi! the industry's standard \x038,4Hi! text ever since the 1500s",
                ),
                black_box(100),
            )
        });
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
