use cfg_rs::*;
use std::collections::HashMap;

type R<V> = Result<V, ConfigError>;

#[derive(Debug, FromConfig)]
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
    err: R<u8>,
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

fn main() -> Result<(), ConfigError> {
    let config = Configuration::builder()
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
        .init()?;
    let mut i = 0;
    for name in config.source_names() {
        i += 1;
        println!("{}: {}", i, name);
    }
    let suit = config.get::<ConfigSuit>("suit")?;
    println!("{:?}", suit);
    Ok(())
}
