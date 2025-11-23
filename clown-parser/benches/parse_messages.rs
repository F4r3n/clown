use clown_parser::message::create_message;
use criterion::{Criterion, criterion_group, criterion_main};
use std::hint::black_box;
fn bench_create_message(message: &str) -> anyhow::Result<()> {
    create_message(message.as_bytes())?;
    Ok(())
}

fn criterion_benchmark(c: &mut Criterion) {
    c.bench_function("create message Quit", |b| {
        b.iter(|| bench_create_message(black_box(":Alice QUIT :Quit: Leaving\r\n")))
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
