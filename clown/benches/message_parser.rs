use clown::irc_view::message_parser::to_spans;
use criterion::{Criterion, criterion_group, criterion_main};
use std::hint::black_box;

fn bench_create_spans(message: &str) -> anyhow::Result<()> {
    to_spans(message, None);
    Ok(())
}

fn criterion_benchmark(c: &mut Criterion) {
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
