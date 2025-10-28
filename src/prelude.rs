use std::borrow::Borrow;

use crate::{ConfigError, ConfigValue, Configuration, FromConfig};

/// Macro to generate config instance from a map of key-value pairs.
/// The keys in the map are config keys, e.g. "port".
/// The values in the map are string values, e.g. "8080".
/// This macro will panic if any required config is missing or
/// if any config value cannot be parsed into the expected type.
/// # Example
/// ```rust
/// use cfg_rs::*;
/// #[derive(Debug, FromConfig)]
/// struct AppConfig {
///    port: u16,
///   host: String,
/// }
/// let config: AppConfig = from_static_map!(AppConfig, {
///    "port" => "8080",
///   "host" => "localhost",
/// });
/// assert_eq!(config.port, 8080);
/// assert_eq!(config.host, "localhost");
/// ```
/// Note: This macro is intended for use in tests or examples where
/// you want to quickly create a config instance from inline key-value pairs.
/// It is not recommended for use in production code.
#[macro_export]
macro_rules! from_static_map {
    ( $ty:ty, { $( $key:expr => $value:expr ),* $(,)? } ) => {{
        use $crate::*;
        use std::collections::HashMap;
        let mut config: HashMap<String, String> = HashMap::new();
        $(
            config.insert($key.to_string(), $value.to_string());
        )*
        from_map::<$ty, _, _, _>(config, "").expect("from_static_map failed")
    }};
}

/// Generate config instance from a map of key-value pairs.
/// The keys in the map are full config keys, e.g. "cfg.app.port".
/// The values in the map are string values, e.g. "8080".
/// The `prefix` is used to scope the config keys, e.g. "cfg.app".
/// This function will return an error if any required config is missing or
/// if any config value cannot be parsed into the expected type.
/// # Example
/// ```rust
/// use std::collections::HashMap;
/// use cfg_rs::*;
/// #[derive(Debug, FromConfig)]
/// struct AppConfig {
///     port: u16,
///     host: String,
/// }
/// let mut map = HashMap::new();
/// map.insert("cfg.app.port", "8080");
/// map.insert("cfg.app.host", "localhost");
/// let config: AppConfig = from_map(map, "cfg.app").unwrap();
/// assert_eq!(config.port, 8080);
/// assert_eq!(config.host, "localhost");
/// ```
#[allow(unused_mut)]
pub fn from_map<
    T: FromConfig,
    I: IntoIterator<Item = (K, V)>,
    K: Borrow<str>,
    V: Into<ConfigValue<'static>>,
>(
    map: I,
    prefix: &str,
) -> Result<T, ConfigError> {
    let mut config = Configuration::new().register_kv("default");
    for (k, v) in map {
        config = config.set(k, v);
    }
    let mut config = config.finish()?;
    #[cfg(feature = "rand")]
    {
        config = config.register_random()?;
    }
    config.get(prefix)
}

/// Generate config instance from environment variables.
/// The `prefix` is used to scope the config keys, e.g. "CFG_APP".
/// This function will return an error if any required config is missing or
/// if any config value cannot be parsed into the expected type.
/// # Example
/// ```rust
/// use cfg_rs::*;
/// #[derive(Debug, FromConfig)]
/// struct AppConfig {
///     port: u16,
///     host: String,
/// }
/// std::env::set_var("CFG_APP_PORT", "8080");
/// std::env::set_var("CFG_APP_HOST", "localhost");
/// let config: AppConfig = from_env("CFG_APP").unwrap();
/// assert_eq!(config.port, 8080);
/// assert_eq!(config.host, "localhost");
/// ```
#[allow(unused_mut)]
pub fn from_env<T: FromConfig>(prefix: &str) -> Result<T, ConfigError> {
    let mut config = Configuration::new().register_prefix_env(prefix)?;
    #[cfg(feature = "rand")]
    {
        config = config.register_random()?;
    }
    config.get("")
}

#[cfg_attr(coverage_nightly, coverage(off))]
#[cfg(test)]
mod tests {
    use super::*;
    use crate::FromConfig;
    use crate::{ConfigContext, ConfigValue};
    use std::collections::HashMap;

    #[derive(Debug, PartialEq, FromConfig)]
    struct TestApp {
        port: u16,
        host: String,
    }

    #[test]
    fn test_from_map_happy_path() {
        let mut map = HashMap::new();
        map.insert("cfg.app.port", "8080");
        map.insert("cfg.app.host", "localhost");

        let cfg: TestApp = from_map(map, "cfg.app").expect("from_map failed");
        assert_eq!(
            cfg,
            TestApp {
                port: 8080,
                host: "localhost".to_string()
            }
        );
    }

    #[test]
    fn test_from_env_happy_path() {
        // Use a unique prefix to avoid colliding with other env vars
        let prefix = "TEST_APP";
        std::env::set_var("TEST_APP_PORT", "9090");
        std::env::set_var("TEST_APP_HOST", "127.0.0.1");

        let cfg: TestApp = from_env(prefix).expect("from_env failed");
        assert_eq!(
            cfg,
            TestApp {
                port: 9090,
                host: "127.0.0.1".to_string()
            }
        );

        // Clean up
        std::env::remove_var("TEST_APP_PORT");
        std::env::remove_var("TEST_APP_HOST");
    }

    #[test]
    fn test_load_from_map_macro_happy_path() {
        // Use the macro to construct TestApp from inline kvs
        let app: TestApp = from_static_map!(TestApp, {
            "port" => "8080",
            "host" => "localhost",
        });

        assert_eq!(
            app,
            TestApp {
                port: 8080,
                host: "localhost".to_string()
            }
        );
    }

    #[test]
    fn test_load_from_map_macro_single_entry() {
        // Single entry form should also work
        // host is missing so deriving FromConfig would error; instead test getting a struct with only port
        #[derive(Debug, PartialEq, FromConfig)]
        struct OnlyPort {
            port: u16,
        }
        let only: OnlyPort = from_static_map!(OnlyPort, { "port" => "7070" });
        assert_eq!(only, OnlyPort { port: 7070 });
    }
}
