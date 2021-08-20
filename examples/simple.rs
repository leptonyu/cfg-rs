use cfg_rs::*;

#[derive(Debug, FromConfig)]
struct Hello {
    json: String,
    toml: String,
    yaml: String,
    #[config(default = "${random.u8}")]
    rand: u64,
}

fn main() -> Result<(), ConfigError> {
    let config = Configuration::builder().set("key", "value").init()?;
    let mut i = 0;
    for name in config.source_names() {
        i += 1;
        println!("{}: {}", i, name);
    }
    let hello = config.get::<Hello>("hello")?;
    println!("{:?}", hello);
    Ok(())
}
