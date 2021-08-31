use cfg_rs::*;

fn main() -> Result<(), ConfigError> {
    let config = Configuration::with_predefined_builder()
        .set_profile("dev")
        .init()?;
    let toml: String = config.get("hello.toml")?;
    // app-dev.toml
    // app.toml
    // will be loaded, app-dev.toml has higher priority.
    // should print "toml-dev"
    println!("{}", toml);
    Ok(())
}
