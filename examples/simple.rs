use cfg_rs::*;
use std::{collections::HashMap, env::set_var};

type R<V> = Result<V, ConfigError>;

#[allow(dead_code)]
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
    err: R<u8>,
    #[config(default = "${random.u64}")]
    rand: u64,
}

#[allow(dead_code)]
#[derive(Debug, FromConfig)]
struct FloatSuit {
    v1: f32,
    v2: f64,
}

#[allow(dead_code)]
#[derive(Debug, FromConfig)]
struct IntSuit {
    v1: u8,
    v2: u16,
    v3: u32,
}

fn main() -> Result<(), ConfigError> {
    set_var("RUST_LOG", "trace");
    env_logger::init();
    // This example need feature full to enable toml/yaml/json source, and load them from app.toml/yaml/json.
    let config = Configuration::with_predefined_builder()
        .set_profile("dev")
        .init()?;
    let mut i = 0;
    for name in config.source_names() {
        i += 1;
        println!("{}: {}", i, name);
    }
    let hello = config.get_predefined::<ConfigSuit>()?;
    println!("{:?}", hello);
    Ok(())
}
