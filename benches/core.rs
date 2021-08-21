use std::collections::HashMap;

use cfg_rs::*;
use criterion::{black_box, criterion_group, criterion_main, Criterion};

#[derive(Debug, FromConfig)]
#[config(prefix = "app")]
struct AppConfig {
    #[config(default = "app")]
    name: String,
    dir: Option<String>,
    profile: Option<String>,
}

#[derive(Debug, FromConfig)]
#[config(prefix = "suit")]
struct ConfigSuit {
    #[config(name = "val")]
    int: IntSuit,
    arr: Vec<String>,
    brr: Vec<Vec<String>>,
    #[config(name = "val")]
    map: HashMap<String, usize>,
    #[config(name = "map")]
    bap: HashMap<String, Vec<bool>>,
    crr: Vec<FloatSuit>,
    err: Result<u8, ConfigError>,
}
#[derive(Debug, FromConfig)]
struct FloatSuit {
    v1: f32,
    v2: f64,
}

#[derive(Debug, FromConfig)]
struct IntSuit {
    v1: u8,
    v2: u16,
    v3: u32,
}

fn criterion_benchmark(c: &mut Criterion) {
    c.bench_function("app1", |b| b.iter(|| Configuration::init()));

    let env = Configuration::builder()
        .set("hello", "world")
        .set("hello.a1.b2.c3.d4.e5.f6.f7", "world")
        .set("rand", "${random.u8}")
        .set("suit.val.v1", "1")
        .set("suit.val.v2", "2")
        .set("suit.val.v3", "3")
        .set("suit.arr[0]", "a0")
        .set("suit.arr[1]", "a1")
        .set("suit.arr[2]", "a2")
        .set("suit.map.b1[0]", "true")
        .set("suit.map.b2[0]", "true")
        .set("suit.map.b2[1]", "false")
        .set("suit.crr[0].v1", "1.0")
        .set("suit.crr[0].v2", "2.0")
        .set("suit.brr[0][0]", "b00")
        .init()
        .unwrap();

    c.bench_function("conf0", |b| b.iter(|| env.get::<AppConfig>("suit")));
    c.bench_function("conf1", |b| b.iter(|| env.get_predefined::<AppConfig>()));
    c.bench_function("conf2", |b| b.iter(|| env.get_predefined::<ConfigSuit>()));

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
