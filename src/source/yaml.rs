//! Yaml config source.
use yaml_rust::YamlLoader;

use super::{memory::HashSourceBuilder, SourceAdaptor, SourceLoader};
use crate::ConfigError;

impl SourceAdaptor for yaml_rust::Yaml {
    fn load(self, source: &mut HashSourceBuilder<'_>) -> Result<(), ConfigError> {
        match self {
            yaml_rust::Yaml::Real(v) => source.insert(v),
            yaml_rust::Yaml::Integer(v) => source.insert(v),
            yaml_rust::Yaml::String(v) => source.insert(v),
            yaml_rust::Yaml::Boolean(v) => source.insert(v),
            yaml_rust::Yaml::Array(v) => source.insert_array(v)?,
            yaml_rust::Yaml::Hash(v) => source.insert_map(
                v.into_iter()
                    .filter_map(|(k, v)| k.as_str().map(|k| (k.to_string(), v))),
            )?,
            _ => {}
        }
        Ok(())
    }
}

impl SourceAdaptor for Yaml {
    fn load(self, builder: &mut HashSourceBuilder<'_>) -> Result<(), ConfigError> {
        for y in self.0 {
            y.load(builder)?;
        }
        Ok(())
    }
}

impl SourceLoader for Yaml {
    type Adaptor = Yaml;
    fn create_loader(content: &str) -> Result<Self::Adaptor, ConfigError> {
        Ok(Yaml(YamlLoader::load_from_str(content)?))
    }

    fn file_extensions() -> Vec<&'static str> {
        vec!["yaml", "yml"]
    }
}

/// Yaml source.
#[allow(missing_debug_implementations)]
pub struct Yaml(Vec<yaml_rust::Yaml>);

/// Inline yaml file macro function, return Result<[ConfigSource](./trait.ConfigSource.html), [`ConfigError`]>.
#[macro_export]
#[cfg_attr(docsrs, doc(cfg(feature = "yaml")))]
macro_rules! inline_yaml {
    ($path:literal) => {
        crate::inline_config_source!(crate::source::yaml::Yaml: $path)
    };
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::test::source_test_suit;

    #[test]
    #[allow(unused_qualifications)]
    fn inline_test() -> Result<(), ConfigError> {
        source_test_suit(inline_yaml!("../../app.yaml")?)
    }
}
