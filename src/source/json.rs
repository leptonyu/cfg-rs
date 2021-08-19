//! Json config source.

use crate::ConfigError;

use super::{file::FileConfigSource, memory::PrefixHashSource};
pub use json::JsonValue;

impl FileConfigSource for JsonValue {
    fn load(content: &str) -> Result<Self, ConfigError> {
        Ok(json::parse(content)?)
    }

    fn ext() -> &'static str {
        "json"
    }

    fn push_value(self, source: &mut PrefixHashSource<'_>) {
        match self {
            JsonValue::String(v) => source.insert(v),
            JsonValue::Short(v) => source.insert(v.as_str().to_string()),
            JsonValue::Number(v) => source.insert(v.to_string()),
            JsonValue::Boolean(v) => source.insert(v),
            JsonValue::Array(v) => {
                let mut i = 0;
                for x in v {
                    source.push(i);
                    i += 1;
                    x.push_value(source);
                    source.pop();
                }
            }
            JsonValue::Object(mut v) => {
                for (k, v) in v.iter_mut() {
                    source.push(k);
                    std::mem::replace(v, JsonValue::Null).push_value(source);
                    source.pop();
                }
            }
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
