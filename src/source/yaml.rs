//! Yaml config source.

use std::collections::HashMap;

use yaml_rust::{Yaml, YamlLoader};

use crate::{ConfigError, ConfigValue};

use super::{file::FileConfigSource, memory::MemoryValue};

/// Yaml source.
#[allow(missing_debug_implementations)]
pub struct Value(Vec<Yaml>);

impl Into<MemoryValue> for Value {
    fn into(self) -> MemoryValue {
        let mut mv = MemoryValue::new();
        for y in self.0 {
            convert(y, &mut mv);
        }
        mv
    }
}

fn convert(y: Yaml, mv: &mut MemoryValue) {
    match y {
        Yaml::Real(v) => mv.value = Some(ConfigValue::Str(v)),
        Yaml::Integer(v) => mv.value = Some(ConfigValue::Int(v)),
        Yaml::String(v) => mv.value = Some(ConfigValue::Str(v)),
        Yaml::Boolean(v) => mv.value = Some(ConfigValue::Bool(v)),
        Yaml::Array(v) => {
            mv.array = v
                .into_iter()
                .map(|v| {
                    let mut mv = MemoryValue::new();
                    convert(v, &mut mv);
                    mv
                })
                .collect()
        }
        Yaml::Hash(v) => {
            mv.table = HashMap::new();
            for (k, x) in v {
                if let Some(k) = k.as_str() {
                    let mut v = MemoryValue::new();
                    convert(x, &mut v);
                    mv.table.insert(k.to_owned(), v);
                }
            }
        }
        _ => {}
    }
}

impl FileConfigSource for Value {
    fn load(content: String) -> Result<MemoryValue, ConfigError> {
        Ok(Value(YamlLoader::load_from_str(&content)?).into())
    }

    fn ext() -> &'static str {
        "yaml"
    }
}
