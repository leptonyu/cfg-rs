//! Json config source.

use super::{memory::ConfigSourceBuilder, ConfigSourceAdaptor, ConfigSourceParser};
use crate::ConfigError;
use serde_json::Value;

/// Json value.
#[doc(hidden)]
pub type Json = Value;

impl ConfigSourceAdaptor for Json {
    fn convert_source(self, source: &mut ConfigSourceBuilder<'_>) -> Result<(), ConfigError> {
        match self {
            Value::Null => {}
            Value::Bool(v) => source.insert(v),
            Value::Number(v) => source.insert(v.to_string()),
            Value::String(v) => source.insert(v),
            Value::Array(v) => source.insert_array(v)?,
            Value::Object(v) => source.insert_map(v)?,
        }
        Ok(())
    }
}

impl ConfigSourceParser for Json {
    type Adaptor = Json;
    fn parse_source(content: &str) -> Result<Self::Adaptor, ConfigError> {
        serde_json::from_str(content).map_err(ConfigError::from_cause)
    }

    fn file_extensions() -> Vec<&'static str> {
        vec!["json"]
    }
}

#[cfg_attr(coverage_nightly, coverage(off))]
#[cfg(test)]
mod test {
    use crate::{inline_source, test::source_test_suit, ConfigError};

    #[test]
    #[allow(unused_qualifications)]
    fn inline_test() -> Result<(), ConfigError> {
        source_test_suit(inline_source!("../../app.json")?)
    }
}
