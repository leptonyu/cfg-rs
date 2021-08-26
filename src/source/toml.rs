//! Toml config source.

use super::{file::FileSourceLoader, memory::HashSourceBuilder, SourceAdaptor, SourceLoader};
use crate::ConfigError;
use toml::Value;

pub type Toml = Value;

impl SourceAdaptor for Toml {
    fn load(self, source: &mut HashSourceBuilder<'_>) -> Result<(), ConfigError> {
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

impl SourceLoader for Toml {
    type Adaptor = Toml;
    fn create_loader(c: &str) -> Result<Self::Adaptor, ConfigError> {
        Ok(toml::from_str::<Value>(c)?)
    }
}

impl FileSourceLoader for Toml {
    fn file_extensions() -> Vec<&'static str> {
        vec!["toml"]
    }
}

/// Inline toml file macro function, return Result<[ConfigSource](./trait.ConfigSource.html), [`ConfigError`]>.
#[macro_export]
#[cfg_attr(docsrs, doc(cfg(feature = "toml")))]
macro_rules! inline_toml {
    ($path:literal) => {
        crate::inline_config_source!(crate::source::toml::Toml: $path)
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
