//! Environment sources.
use std::env::vars;

use crate::{ConfigKey, ConfigSource, ConfigValue};

use super::memory::MemorySource;

/// Simple environment source.
#[derive(Debug)]
pub struct EnvironmentSource(MemorySource);

impl EnvironmentSource {
    /// Create source.
    pub fn new() -> Self {
        let mut body = MemorySource::new("env".to_string());
        for (k, v) in vars() {
            body = body.set(k, v);
        }
        Self(body)
    }
}

impl ConfigSource for EnvironmentSource {
    #[inline]
    fn get_value(&self, key: &ConfigKey<'_>) -> Option<ConfigValue<'_>> {
        self.0.get_value(key)
    }

    #[inline]
    fn collect_keys<'a>(&'a self, prefix: &ConfigKey<'_>, sub: &mut crate::SubKeyList<'a>) {
        self.0.collect_keys(prefix, sub)
    }

    fn name(&self) -> &str {
        "env"
    }
}

/// Prefixed environment source.
#[derive(Debug)]
pub struct EnvironmentPrefixedSource(String, MemorySource);

impl EnvironmentPrefixedSource {
    /// Create prefixed environment source.
    pub fn new(prefix: &str) -> Self {
        let prefix = format!("{}_", prefix.to_uppercase());
        let mut body = MemorySource::new(format!("env:{}*", prefix));
        for (k, v) in vars() {
            if let Some(kk) = k.strip_prefix(&prefix) {
                body = body.set(kk.to_lowercase().replace('_', "."), v);
            }
        }
        Self(prefix, body)
    }
}

impl ConfigSource for EnvironmentPrefixedSource {
    #[inline]
    fn get_value(&self, key: &ConfigKey<'_>) -> Option<ConfigValue<'_>> {
        self.1.get_value(key)
    }

    #[inline]
    fn collect_keys<'a>(&'a self, prefix: &ConfigKey<'_>, sub: &mut crate::SubKeyList<'a>) {
        self.1.collect_keys(prefix, sub)
    }

    #[inline]
    fn name(&self) -> &str {
        self.1.name()
    }
}

#[cfg(test)]
mod test {
    use std::{collections::HashMap, env::set_var};

    use crate::test::TestConfigExt;

    use super::EnvironmentPrefixedSource;

    #[test]
    fn env_key_test() {
        set_var("HELLO_WORLD", "hello");

        let config = EnvironmentPrefixedSource::new("hello").new_config();

        let value = config.get::<String>("world");
        assert_eq!("hello", value.unwrap());

        let value = config.get::<String>("world2");
        assert_eq!(true, value.is_err());
    }

    #[test]
    fn env_arr_test() {
        set_var("HELLO_ARR_0", "h0");
        set_var("HELLO_ARR_1", "h1");
        set_var("HELLO_BRR_1", "b0");
        set_var("HELLO_CRR_0_0", "c0");
        let config = EnvironmentPrefixedSource::new("hello").new_config();

        let value = config.get::<Vec<String>>("arr");
        assert_eq!(vec!["h0", "h1"], value.unwrap());

        let value = config.get::<Vec<String>>("arr2");
        let vs: Vec<String> = vec![];
        assert_eq!(vs, value.unwrap());

        let value = config.get::<Vec<String>>("brr");
        assert_eq!(true, value.is_err());

        let value = config.get::<Vec<Option<String>>>("brr");
        let vs: Vec<Option<String>> = vec![None, Some("b0".to_string())];
        assert_eq!(vs, value.unwrap());

        let value = config.get::<Vec<Vec<String>>>("rrr");
        let vs: Vec<Vec<String>> = vec![];
        assert_eq!(vs, value.unwrap());

        let value = config.get::<Vec<Vec<String>>>("crr");
        let vs: Vec<Vec<String>> = vec![vec!["c0".to_string()]];
        assert_eq!(vs, value.unwrap());
    }

    #[test]
    fn env_map_test() {
        set_var("HELLO_MAP_0", "h0");
        set_var("HELLO_MAP_K1", "v1");
        set_var("HELLO_MAP_K2", "v2");
        let config = EnvironmentPrefixedSource::new("hello").new_config();
        let value = config.get::<HashMap<String, String>>("map");
        let mut map: HashMap<String, String> = HashMap::new();
        map.insert("k1".into(), "v1".into());
        map.insert("k2".into(), "v2".into());
        assert_eq!(map, value.unwrap());

        let value = config.get::<HashMap<String, String>>("map2");
        let map: HashMap<String, String> = HashMap::new();
        assert_eq!(map, value.unwrap());
    }
}
