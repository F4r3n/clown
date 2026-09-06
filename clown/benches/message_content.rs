use clown::message_irc::message_content::MessageContent;
use criterion::{Criterion, criterion_group, criterion_main};
use std::hint::black_box;

fn criterion_benchmark(c: &mut Criterion) {
    c.bench_function("create_rows wrapped", |b| {
        let msg = MessageContent::message(
            Some("nickname".to_string()),
            "Lorem Ipsum is simply dummy text of the printing and typesetting industry. \
             Lorem Ipsum has been the industry's standard dummy text ever since the 1500s"
                .to_string(),
        );
        let total = msg.wrapped_line_count(40);
        b.iter(|| {
            black_box(&msg)
                .create_rows(
                    black_box(40),
                    None,
                    Some(&clown::message_irc::message_content::TimeFormat::Hour),
                    black_box(10),
                    black_box(0),
                    black_box(total),
                )
                .count()
        });
    });

    // The scrolled case: only a slice of a long message is on screen, so
    // `create_rows` should only build that slice.
    c.bench_function("create_rows windowed", |b| {
        let msg = MessageContent::message(
            Some("nickname".to_string()),
            "Lorem Ipsum is simply dummy text of the printing and typesetting industry. \
             Lorem Ipsum has been the industry's standard dummy text ever since the 1500s"
                .to_string(),
        );
        b.iter(|| {
            black_box(&msg)
                .create_rows(
                    black_box(40),
                    None,
                    Some(&clown::message_irc::message_content::TimeFormat::Hour),
                    black_box(10),
                    black_box(2),
                    black_box(2),
                )
                .count()
        });
    });

    c.bench_function("create_rows single line", |b| {
        let msg =
            MessageContent::message(Some("nickname".to_string()), "short message".to_string());
        b.iter(|| {
            black_box(&msg)
                .create_rows(
                    black_box(120),
                    None,
                    Some(&clown::message_irc::message_content::TimeFormat::Hour),
                    black_box(10),
                    black_box(0),
                    black_box(1),
                )
                .count()
        });
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
