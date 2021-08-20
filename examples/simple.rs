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
    let config = Configuration::build(Ok)?;
    for name in config.source_names() {
        println!("{}", name);
    }
    let hello = config.get::<Hello>("hello")?;
    println!("{:?}", hello);
    Ok(())
}