use clown::irc_view::message_parser::{get_width_without_format, is_string_plain, to_spans};
use clown::irc_view::textwrapper::{wrap_content, wrapped_line_count};
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

fn bench_wrap_line_count(message: &str, width: usize) -> anyhow::Result<()> {
    wrapped_line_count(message, width);
    Ok(())
}

fn bench_get_size_witout_format(message: &str) -> anyhow::Result<()> {
    get_width_without_format(message);
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
        });
        b.iter(|| {
            bench_create_spans(black_box(
                "Lorem Ipsum is simply \x038,4Hi! text of the printing and \x038,4Hi! industry.
            Lorem Ipsum has \x038,4Hi! the industry's standard \x038,4Hi! text ever since the 1500s",
            ))
        });

    });

    c.bench_function("get_size_witout_format", |b| {
        b.iter(|| {
            bench_get_size_witout_format(black_box(
                "Lorem Ipsum is simply \x038,4Hi! text of the printing and \x038,4Hi! industry.
            Lorem Ipsum has \x038,4Hi! the industry's standard \x038,4Hi! text ever since the 1500s",
            ))
        });

        b.iter(|| {
            bench_get_size_witout_format(black_box(
                "Lorem Ipsum is simply dummy text of the printing and typesetting industry.
            Lorem Ipsum has been the industry's standard dummy text ever since the 1500s",
            ))
        });
    });

    c.bench_function("Wrap text", |b| {
        b.iter(|| {
            bench_wrap_line_count(black_box(
                "Lorem Ipsum is simply \x038,4Hi! text of the printing and \x038,4Hi! industry.
            Lorem Ipsum has \x038,4Hi! the industry's standard \x038,4Hi! text ever since the 1500s"
            ), black_box(10))
        });

        b.iter(|| {
            bench_wrap_line_count(black_box(
                "Lorem Ipsum is simply \x038,4Hi! text of the printing and \x038,4Hi! industry.
            Lorem Ipsum has \x038,4Hi! the industry's standard \x038,4Hi! text ever since the 1500s"
            ), black_box(100))
        });
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
