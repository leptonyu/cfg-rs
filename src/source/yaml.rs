//! Yaml config source.

use yaml_rust::{Yaml, YamlLoader};

use crate::ConfigError;

use super::{file::FileConfigSource, memory::PrefixHashSource};

/// Yaml source.
#[allow(missing_debug_implementations)]
pub struct Value(Vec<Yaml>);

fn convert(y: Yaml, source: &mut PrefixHashSource<'_>) {
    match y {
        Yaml::Real(v) => source.insert(v),
        Yaml::Integer(v) => source.insert(v),
        Yaml::String(v) => source.insert(v),
        Yaml::Boolean(v) => source.insert(v),
        Yaml::Array(v) => {
            let mut i = 0;
            for x in v {
                source.push(i);
                i += 1;
                convert(x, source);
                source.pop();
            }
        }
        Yaml::Hash(v) => {
            for (k, v) in v {
                if let Some(k) = k.as_str() {
                    source.push(k);
                    convert(v, source);
                    source.pop();
                }
            }
        }
        _ => {}
    }
}

impl FileConfigSource for Value {
    fn load(content: String) -> Result<Self, ConfigError> {
        Ok(Value(YamlLoader::load_from_str(&content)?))
    }

    fn ext() -> &'static str {
        "yaml"
    }

    fn push_value(self, source: &mut PrefixHashSource<'_>) {
        for y in self.0 {
            convert(y, source);
        }
    }
}
