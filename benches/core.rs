use cfg_rs::*;
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn criterion_benchmark(c: &mut Criterion) {
    let env = Configuration::builder()
        .set("hello", "world")
        .set("hello.a1.b2.c3.d4.e5.f6.f7", "world")
        .set("rand", "${random.u8}")
        .init()
        .unwrap();

    c.bench_function("hello1", |b| {
        b.iter(|| env.get::<String>(black_box("hello")))
    });

    c.bench_function("hello2", |b| {
        b.iter(|| env.get::<Option<String>>(black_box("hello")))
    });

    c.bench_function("long1", |b| {
        b.iter(|| env.get::<String>(black_box("hello.a1.b2.c3.d4.e5.f6.f7")))
    });

    c.bench_function("long2", |b| {
        b.iter(|| env.get::<Option<String>>(black_box("hello.a1.b2.c3.d4.e5.f6.f7")))
    });

    c.bench_function("not", |b| {
        b.iter(|| env.get::<Option<String>>(black_box("world")))
    });

    c.bench_function("rand1", |b| {
        b.iter(|| env.get::<String>(black_box("random.u8")))
    });
    c.bench_function("rand2", |b| b.iter(|| env.get::<String>(black_box("rand"))));
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
