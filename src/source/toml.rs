//! Toml config source.

use crate::{ConfigError, ConfigValue};
pub use toml::Value;

use super::{file::FileConfigSource, memory::MemoryValue};

impl Into<MemoryValue> for Value {
    fn into(self) -> MemoryValue {
        let mut mv = MemoryValue::new();
        match self {
            Value::String(v) => mv.value = Some(ConfigValue::Str(v)),
            Value::Integer(v) => mv.value = Some(ConfigValue::Int(v)),
            Value::Float(v) => mv.value = Some(ConfigValue::Float(v)),
            Value::Boolean(v) => mv.value = Some(ConfigValue::Bool(v)),
            Value::Datetime(v) => mv.value = Some(ConfigValue::Str(v.to_string())),
            Value::Array(v) => mv.array = v.into_iter().map(|v| v.into()).collect(),
            Value::Table(v) => mv.table = v.into_iter().map(|(k, v)| (k, v.into())).collect(),
        }
        mv
    }
}

impl FileConfigSource for Value {
    fn load(content: String) -> Result<MemoryValue, ConfigError> {
        Ok(toml::from_str::<Value>(&content)?.into())
    }

    fn ext() -> &'static str {
        "toml"
    }
}
