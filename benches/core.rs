use cfg_rs::*;
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn criterion_benchmark(c: &mut Criterion) {
    let env = Configuration::build(|f| Ok(f.set("hello", "world"))).unwrap();

    c.bench_function("hello1", |b| {
        b.iter(|| env.get::<String>(black_box("hello")))
    });

    c.bench_function("hello2", |b| {
        b.iter(|| env.get::<Option<String>>(black_box("hello")))
    });

    c.bench_function("hello3", |b| {
        b.iter(|| env.get::<Option<String>>(black_box("world")))
    });

    c.bench_function("rand", |b| {
        b.iter(|| env.get::<String>(black_box("random.u8")))
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
