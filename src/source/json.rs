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

/// Inline json file macro function, return Result<[ConfigSource](./trait.ConfigSource.html), [`ConfigError`]>.
#[macro_export]
#[cfg_attr(docsrs, doc(cfg(feature = "json")))]
macro_rules! inline_json {
    ($path:literal) => {
        crate::inline_config_source!(crate::source::json::JsonValue: $path)
    };
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::test::source_test_suit;

    #[test]
    #[allow(unused_qualifications)]
    fn inline_test() -> Result<(), ConfigError> {
        source_test_suit(inline_json!("../../app.json")?)
    }
}
