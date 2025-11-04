use clown::irc_view::message_parser::{is_string_plain, to_spans};
use criterion::{Criterion, criterion_group, criterion_main};
use std::hint::black_box;

fn bench_create_spans(message: &str) -> anyhow::Result<()> {
    to_spans(message, None);
    Ok(())
}

fn bench_is_message_plain(message: &str) -> anyhow::Result<()> {
    is_string_plain(message);
    Ok(())
}

fn criterion_benchmark(c: &mut Criterion) {
    c.bench_function("Check message is plain", |b| {
        b.iter(|| {
            bench_is_message_plain(black_box(
                "Lorem Ipsum is simply dummy text of the printing and typesetting industry.
            Lorem Ipsum has been the industry's standard dummy text ever since the 1500s",
            ))
        });
    });

    c.bench_function("create spans", |b| {
        b.iter(|| {
            bench_create_spans(black_box(
                "Lorem Ipsum is simply dummy text of the printing and typesetting industry.
            Lorem Ipsum has been the industry's standard dummy text ever since the 1500s",
            ))
        })
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
