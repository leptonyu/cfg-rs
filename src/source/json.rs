//! Json config source.

use super::{memory::ConfigSourceBuilder, ConfigSourceAdaptor, ConfigSourceParser};
use crate::ConfigError;
use json::JsonValue;

pub type Json = JsonValue;

impl ConfigSourceAdaptor for Json {
    fn convert_source(self, source: &mut ConfigSourceBuilder<'_>) -> Result<(), ConfigError> {
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

impl ConfigSourceParser for Json {
    type Adaptor = Json;
    fn parse_source(content: &str) -> Result<Self::Adaptor, ConfigError> {
        Ok(json::parse(content)?)
    }

    fn file_extensions() -> Vec<&'static str> {
        vec!["json"]
    }
}

#[cfg(test)]
mod test {
    use crate::{inline_source, test::source_test_suit, ConfigError};

    #[test]
    #[allow(unused_qualifications)]
    fn inline_test() -> Result<(), ConfigError> {
        source_test_suit(inline_source!("../../app.json")?)
    }
}
