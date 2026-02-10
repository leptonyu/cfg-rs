//! Toml config source.

use super::{memory::ConfigSourceBuilder, ConfigSourceAdaptor, ConfigSourceParser};
use crate::ConfigError;
use toml::Value;

pub type Toml = Value;

impl ConfigSourceAdaptor for Toml {
    fn convert_source(self, source: &mut ConfigSourceBuilder<'_>) -> Result<(), ConfigError> {
        match self {
            Value::String(v) => source.insert(v),
            Value::Integer(v) => source.insert(v),
            Value::Float(v) => source.insert(v),
            Value::Boolean(v) => source.insert(v),
            Value::Datetime(v) => source.insert(v.to_string()),
            Value::Array(v) => source.insert_array(v)?,
            Value::Table(v) => source.insert_map(v)?,
        }
        Ok(())
    }
}

impl ConfigSourceParser for Toml {
    type Adaptor = Toml;
    fn parse_source(c: &str) -> Result<Self::Adaptor, ConfigError> {
        Ok(toml::from_str::<Value>(c).map_err(ConfigError::from_cause)?)
    }

    fn file_extensions() -> Vec<&'static str> {
        vec!["toml", "tml"]
    }
}

#[cfg_attr(coverage_nightly, coverage(off))]
#[cfg(test)]
mod test {
    use super::*;
    use crate::{source::inline_source, test::source_test_suit};

    #[test]
    #[allow(unused_qualifications)]
    fn inline_test() -> Result<(), ConfigError> {
        source_test_suit(inline_source!("../../app.toml")?)
    }
}
