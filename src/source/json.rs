//! Json config source.

use std::collections::HashMap;

use crate::{ConfigError, ConfigValue};

use super::{file::FileConfigSource, memory::MemoryValue};
pub use json::JsonValue;

impl Into<MemoryValue> for JsonValue {
    fn into(self) -> MemoryValue {
        let mut mv = MemoryValue::new();
        match self {
            JsonValue::String(v) => mv.value = Some(ConfigValue::Str(v)),
            JsonValue::Short(v) => mv.value = Some(ConfigValue::Str(v.as_str().to_owned())),
            JsonValue::Number(v) => mv.value = Some(ConfigValue::Str(v.to_string())),
            JsonValue::Boolean(v) => mv.value = Some(ConfigValue::Bool(v)),
            JsonValue::Array(v) => mv.array = v.into_iter().map(|v| v.into()).collect(),
            JsonValue::Object(mut v) => {
                mv.table = HashMap::new();
                for (k, v) in v.iter_mut() {
                    mv.table
                        .insert(k.to_string(), std::mem::replace(v, JsonValue::Null).into());
                }
            }
            JsonValue::Null => {}
        }
        mv
    }
}

impl FileConfigSource for JsonValue {
    fn load(content: String) -> Result<MemoryValue, ConfigError> {
        Ok(json::parse(&content)?.into())
    }

    fn ext() -> &'static str {
        "json"
    }
}
