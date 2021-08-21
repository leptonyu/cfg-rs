//! Toml config source.

use crate::ConfigError;
pub use toml::Value;

use super::{file::FileConfigSource, memory::HashSourceBuilder};

impl FileConfigSource for Value {
    fn load(content: &str) -> Result<Self, ConfigError> {
        Ok(toml::from_str::<Value>(content)?)
    }

    fn push_value(self, source: &mut HashSourceBuilder<'_>) {
        match self {
            Value::String(v) => source.insert(v),
            Value::Integer(v) => source.insert(v),
            Value::Float(v) => source.insert(v),
            Value::Boolean(v) => source.insert(v),
            Value::Datetime(v) => source.insert(v.to_string()),
            Value::Array(v) => source.insert_array(v),
            Value::Table(v) => source.insert_map(v),
        }
    }

    fn ext() -> &'static str {
        "toml"
    }
}

/// Inline toml file macro function, return Result<[ConfigSource](./trait.ConfigSource.html), [`ConfigError`]>.
#[macro_export]
#[cfg_attr(docsrs, doc(cfg(feature = "toml")))]
macro_rules! inline_toml {
    ($path:literal) => {
        crate::inline_config_source!(crate::source::toml::Value: $path)
    };
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::test::source_test_suit;

    #[test]
    #[allow(unused_qualifications)]
    fn inline_test() -> Result<(), ConfigError> {
        source_test_suit(inline_toml!("../../app.toml")?)
    }
}
