use crate::source::*;
use cfg_rs::*;
use std::time::SystemTime;

struct Version;

impl ConfigSource for Version {
    fn name(&self) -> &str {
        "version"
    }
    fn load(&self, b: &mut ConfigSourceBuilder<'_>) -> Result<(), ConfigError> {
        let v = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_micros();
        b.set("version", v);
        Ok(())
    }
    fn allow_refresh(&self) -> bool {
        true
    }
    fn refreshable(&self) -> Result<bool, ConfigError> {
        Ok(true)
    }
}

fn main() -> Result<(), ConfigError> {
    let config = Configuration::new().register_source(Version).unwrap();

    let c = config.get::<RefValue<String>>("version").unwrap();

    for _ in 0..10 {
        config.refresh_ref().unwrap();
        println!("{}", c.get().unwrap());
    }
    Ok(())
}
