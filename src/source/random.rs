//! Random source.
use crate::{ConfigKey, ConfigSource, ConfigValue, SubKeyList};

/// Random source.
#[allow(missing_debug_implementations, missing_copy_implementations)]
pub struct Random;

impl ConfigSource for Random {
    fn get_value(&self, key: &ConfigKey<'_>) -> Option<ConfigValue<'_>> {
        match key.as_str() {
            "random.u8" => Some(ConfigValue::Int(rand::random::<u8>() as i64)),
            "random.u16" => Some(ConfigValue::Int(rand::random::<u16>() as i64)),
            "random.u32" => Some(ConfigValue::Int(rand::random::<u32>() as i64)),
            "random.u64" => Some(ConfigValue::Str(rand::random::<u64>().to_string())),
            "random.u128" => Some(ConfigValue::Str(rand::random::<u128>().to_string())),
            "random.i8" => Some(ConfigValue::Int(rand::random::<i8>() as i64)),
            "random.i16" => Some(ConfigValue::Int(rand::random::<i16>() as i64)),
            "random.i32" => Some(ConfigValue::Int(rand::random::<i32>() as i64)),
            "random.i64" => Some(ConfigValue::Int(rand::random::<i64>())),
            "random.i128" => Some(ConfigValue::Str(rand::random::<i128>().to_string())),
            "random.usize" => Some(ConfigValue::Str(rand::random::<usize>().to_string())),
            "random.isize" => Some(ConfigValue::Str(rand::random::<isize>().to_string())),
            _ => None,
        }
    }

    fn collect_keys<'a>(&'a self, prefix: &ConfigKey<'_>, sub: &mut SubKeyList<'a>) {
        if prefix.as_str() == "random" {
            sub.insert_str("u8");
            sub.insert_str("u16");
            sub.insert_str("u32");
            sub.insert_str("u64");
            sub.insert_str("u128");
            sub.insert_str("i8");
            sub.insert_str("i16");
            sub.insert_str("i32");
            sub.insert_str("i64");
            sub.insert_str("i128");
            sub.insert_str("usize");
            sub.insert_str("isize");
        }
    }

    fn name(&self) -> &str {
        "random"
    }

    fn is_empty(&self) -> bool {
        false
    }
}

#[cfg(test)]
mod test {

    use crate::Configuration;

    use super::Random;

    #[test]
    fn env_test() {
        let config = Configuration::new().register_source(Random);
        let a = config.get::<u128>("random.u128").unwrap();
        let b = config.get::<u128>("random.u128").unwrap();
        assert_ne!(a, b);
    }
}
