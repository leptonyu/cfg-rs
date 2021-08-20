//! Yaml config source.

use yaml_rust::{Yaml, YamlLoader};

use crate::ConfigError;

use super::{file::FileConfigSource, memory::HashSourceBuilder};

impl FileConfigSource for Yaml {
    fn load(_content: &str) -> Result<Self, ConfigError> {
        unimplemented!()
    }

    fn ext() -> &'static str {
        "yaml"
    }

    fn push_value(self, source: &mut HashSourceBuilder<'_>) {
        match self {
            Yaml::Real(v) => source.insert(v),
            Yaml::Integer(v) => source.insert(v),
            Yaml::String(v) => source.insert(v),
            Yaml::Boolean(v) => source.insert(v),
            Yaml::Array(v) => source.insert_array(v),
            Yaml::Hash(v) => source.insert_map(
                v.into_iter()
                    .filter_map(|(k, v)| k.as_str().map(|k| (k.to_string(), v))),
            ),
            _ => {}
        }
    }
}
/// Yaml source.
#[allow(missing_debug_implementations)]
pub struct Value(Vec<Yaml>);

impl FileConfigSource for Value {
    fn load(content: &str) -> Result<Self, ConfigError> {
        Ok(Value(YamlLoader::load_from_str(content)?))
    }

    fn ext() -> &'static str {
        "yaml"
    }

    fn push_value(self, source: &mut HashSourceBuilder<'_>) {
        for y in self.0 {
            y.push_value(source);
        }
    }
}