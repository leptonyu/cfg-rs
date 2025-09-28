//! Yaml config source.
use yaml_rust2::YamlLoader;

use super::{memory::ConfigSourceBuilder, ConfigSourceAdaptor, ConfigSourceParser};
use crate::ConfigError;

impl ConfigSourceAdaptor for yaml_rust2::Yaml {
    fn convert_source(self, source: &mut ConfigSourceBuilder<'_>) -> Result<(), ConfigError> {
        match self {
            yaml_rust2::Yaml::Real(v) => source.insert(v),
            yaml_rust2::Yaml::Integer(v) => source.insert(v),
            yaml_rust2::Yaml::String(v) => source.insert(v),
            yaml_rust2::Yaml::Boolean(v) => source.insert(v),
            yaml_rust2::Yaml::Array(v) => source.insert_array(v)?,
            yaml_rust2::Yaml::Hash(v) => source.insert_map(
                v.into_iter()
                    .filter_map(|(k, v)| k.as_str().map(|k| (k.to_string(), v))),
            )?,
            _ => {}
        }
        Ok(())
    }
}

impl ConfigSourceAdaptor for Yaml {
    fn convert_source(self, builder: &mut ConfigSourceBuilder<'_>) -> Result<(), ConfigError> {
        for y in self.0 {
            y.convert_source(builder)?;
        }
        Ok(())
    }
}

impl ConfigSourceParser for Yaml {
    type Adaptor = Yaml;
    fn parse_source(content: &str) -> Result<Self::Adaptor, ConfigError> {
        Ok(Yaml(YamlLoader::load_from_str(content)?))
    }

    fn file_extensions() -> Vec<&'static str> {
        vec!["yaml", "yml"]
    }
}

/// Yaml source.
#[allow(missing_debug_implementations)]
pub struct Yaml(Vec<yaml_rust2::Yaml>);

#[cfg(test)]
mod test {
    use super::*;
    use crate::{source::inline_source, test::source_test_suit};

    #[test]
    #[allow(unused_qualifications)]
    fn inline_test() -> Result<(), ConfigError> {
        source_test_suit(inline_source!("../../app.yaml")?)
    }
}
