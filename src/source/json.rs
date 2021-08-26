//! Json config source.

use super::{memory::HashSourceBuilder, SourceAdaptor, SourceLoader};
use crate::ConfigError;
use json::JsonValue;

pub type Json = JsonValue;

impl SourceAdaptor for Json {
    fn load(self, source: &mut HashSourceBuilder<'_>) -> Result<(), ConfigError> {
        match self {
            JsonValue::String(v) => source.insert(v),
            JsonValue::Short(v) => source.insert(v.as_str().to_string()),
            JsonValue::Number(v) => source.insert(v.to_string()),
            JsonValue::Boolean(v) => source.insert(v),
            JsonValue::Array(v) => source.insert_array(v)?,
            JsonValue::Object(mut v) => source.insert_map(
                v.iter_mut()
                    .map(|(k, v)| (k, std::mem::replace(v, JsonValue::Null))),
            )?,
            JsonValue::Null => {}
        }
        Ok(())
    }
}

impl SourceLoader for Json {
    type Adaptor = Json;
    fn create_loader(content: &str) -> Result<Self::Adaptor, ConfigError> {
        Ok(json::parse(content)?)
    }

    fn file_extensions() -> Vec<&'static str> {
        vec!["json"]
    }
}

/// Inline json file macro function, return Result<[ConfigSource](./trait.ConfigSource.html), [`ConfigError`]>.
#[macro_export]
#[cfg_attr(docsrs, doc(cfg(feature = "json")))]
macro_rules! inline_json {
    ($path:literal) => {
        crate::inline_config_source!(crate::source::json::Json: $path)
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
