use std::collections::HashMap;

use cfg_rs::*;

#[derive(FromConfig, Debug)]
#[config(prefix = "salak")]
struct Config {
    #[config(default = "world")]
    hello: String,
    world: Option<String>,
    #[config(name = "hello")]
    hey: Option<String>,
    #[config(default = 123)]
    num: u8,
    arr: Vec<u8>,
    map: HashMap<String, u8>,
}

fn main() -> Result<(), ConfigError> {
    let env = Configuration::with_predefined()?;
    for _i in 0..1000 {
        let c = env.get_predefined::<Config>()?;
        if _i % 100 == 0 {
            println!("Round {}: {} - {}", _i, c.hello, c.num);
        }
    }
    Ok(())
}
