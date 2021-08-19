//! Json config source.

use crate::ConfigError;

use super::{file::FileConfigSource, memory::HashSourceBuilder};
pub use json::JsonValue;

impl FileConfigSource for JsonValue {
    fn load(content: &str) -> Result<Self, ConfigError> {
        Ok(json::parse(content)?)
    }

    fn ext() -> &'static str {
        "json"
    }

    fn push_value(self, source: &mut HashSourceBuilder<'_>) {
        match self {
            JsonValue::String(v) => source.insert(v),
            JsonValue::Short(v) => source.insert(v.as_str().to_string()),
            JsonValue::Number(v) => source.insert(v.to_string()),
            JsonValue::Boolean(v) => source.insert(v),
            JsonValue::Array(v) => source.insert_array(v),
            JsonValue::Object(mut v) => source.insert_map(
                v.iter_mut()
                    .map(|(k, v)| (k, std::mem::replace(v, JsonValue::Null))),
            ),
            JsonValue::Null => {}
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{inline_config_source, Configuration};

    #[test]
    fn inline_test() -> Result<(), ConfigError> {
        let v = inline_config_source!(JsonValue: "../../app.json")?;
        let config = Configuration::new().register_source(v);
        assert_eq!("json", config.get::<String>("hello.json")?);
        Ok(())
    }
}
