//! Toml config source.

use crate::ConfigError;
pub use toml::Value;

use super::{file::FileConfigSource, memory::PrefixHashSource};

impl FileConfigSource for Value {
    fn load(content: String) -> Result<Self, ConfigError> {
        Ok(toml::from_str::<Value>(&content)?)
    }

    fn push_value(self, source: &mut PrefixHashSource<'_>) {
        match self {
            Value::String(v) => source.insert(v),
            Value::Integer(v) => source.insert(v),
            Value::Float(v) => source.insert(v),
            Value::Boolean(v) => source.insert(v),
            Value::Datetime(v) => source.insert(v.to_string()),
            Value::Array(v) => {
                let mut i = 0;
                for x in v {
                    source.push(i);
                    i += 1;
                    x.push_value(source);
                    source.pop();
                }
            }
            Value::Table(v) => {
                for (k, v) in v {
                    source.push(k.as_str());
                    v.push_value(source);
                    source.pop();
                }
            }
        }
    }

    fn ext() -> &'static str {
        "toml"
    }
}
