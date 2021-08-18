use std::{
    any::{type_name, Any},
    collections::HashSet,
    env::var,
};

use crate::{
    err::ConfigError,
    key::{ConfigKey, SubKeySeq},
    source::{
        environment::EnvironmentPrefixedSource, layered::LayeredSource, memory::MemorySource,
        register_files, SourceOption,
    },
    value::ConfigValue,
    ConfigSource, FromConfig, FromConfigWithPrefix, SubKeyList,
};

/// Configuration context.
#[allow(missing_debug_implementations)]
pub struct ConfigContext<'a> {
    key: ConfigKey<'a>,
    source: &'a Configuration,
}

#[inline]
fn merge(val: Option<String>, new: &str) -> String {
    match val {
        Some(mut v) => {
            v.push_str(new);
            v
        }
        None => new.to_owned(),
    }
}

fn parse_placeholder<'a>(
    source: &'a Configuration,
    current_key: &ConfigKey<'_>,
    mut value: &str,
    history: &mut HashSet<String>,
) -> Result<Option<ConfigValue<'a>>, ConfigError> {
    let mut stack = vec!["".to_owned()];
    let pat: &[_] = &['$', '\\', '}'];
    while let Some(pos) = value.find(pat) {
        match &value[pos..=pos] {
            "$" => {
                let pos_1 = pos + 1;
                if value.len() == pos_1 || &value[pos_1..=pos_1] != "{" {
                    return Err(ConfigError::ConfigRecursiveError(current_key.to_string()));
                }
                let last = stack.pop();
                stack.push(merge(last, &value[..pos]));
                stack.push("".to_owned());
                value = &value[pos + 2..];
            }
            "\\" => {
                let pos_1 = pos + 1;
                if value.len() == pos_1 {
                    return Err(ConfigError::ConfigRecursiveError(current_key.to_string()));
                }
                let last = stack.pop();
                let mut v = merge(last, &value[..pos]);
                v.push_str(&value[pos_1..=pos_1]);
                stack.push(v);
                value = &value[pos + 2..];
            }
            "}" => {
                let last = stack.pop();
                let v = merge(last, &value[..pos]);
                let (key, def) = match v.find(':') {
                    Some(pos) => (&v[..pos], Some(&v[pos + 1..])),
                    _ => (&v[..], None),
                };
                if !history.insert(key.to_string()) {
                    return Err(ConfigError::ConfigRecursiveError(current_key.to_string()));
                }
                let v = match source
                    .new_context()
                    .do_parse_config::<String, &str>(key, None, history)
                {
                    Err(ConfigError::ConfigNotFound(v)) => match def {
                        Some(v) => v.to_owned(),
                        _ => return Err(ConfigError::ConfigRecursiveNotFound(v)),
                    },
                    ret => ret?,
                };
                history.remove(key);
                let v = merge(stack.pop(), &v);
                stack.push(v);
                value = &value[pos + 1..];
            }
            _ => return Err(ConfigError::ConfigRecursiveError(current_key.to_string())),
        }
    }
    if let Some(mut v) = stack.pop() {
        if stack.is_empty() {
            v.push_str(value);
            return Ok(Some(v.into()));
        }
    }
    Ok(None)
}

impl<'a> ConfigContext<'a> {
    #[inline]
    pub(crate) fn do_parse_config<T: FromConfig, K: Into<SubKeySeq<'a>>>(
        &mut self,
        partial_key: K,
        default_value: Option<ConfigValue<'_>>,
        history: &mut HashSet<String>,
    ) -> Result<T, ConfigError> {
        self.key.push(partial_key);
        let value = match self.source.internal.get_value(&self.key).or(default_value) {
            Some(ConfigValue::Str(s)) => parse_placeholder(self.source, &self.key, &s, history)?,
            Some(ConfigValue::StrRef(s)) => parse_placeholder(self.source, &self.key, s, history)?,
            v => v,
        };
        let v = T::from_config(self, value);
        self.key.pop();
        v
    }

    /// Parse config with sub key.
    #[inline]
    pub fn parse_config<T: FromConfig>(
        &mut self,
        partial_key: &'a str,
        default_value: Option<ConfigValue<'_>>,
    ) -> Result<T, ConfigError> {
        self.do_parse_config(partial_key, default_value, &mut HashSet::new())
    }

    /// Get current key in contxt.
    #[inline]
    pub fn current_key(&self) -> String {
        self.key.to_string()
    }

    #[inline]
    pub(crate) fn type_mismatch<T: Any>(&self, value: &ConfigValue<'_>) -> ConfigError {
        let tp = match value {
            ConfigValue::StrRef(_) => "String",
            ConfigValue::Str(_) => "String",
            ConfigValue::Int(_) => "Integer",
            ConfigValue::Float(_) => "Float",
            ConfigValue::Bool(_) => "Bool",
        };
        ConfigError::ConfigTypeMismatch(self.current_key(), tp, type_name::<T>())
    }

    #[inline]
    pub(crate) fn not_found(&self) -> ConfigError {
        ConfigError::ConfigNotFound(self.current_key())
    }

    #[inline]
    pub(crate) fn parse_error(&self, value: &str) -> ConfigError {
        ConfigError::ConfigParseError(self.current_key(), value.to_owned())
    }

    pub(crate) fn collect_keys(&self) -> SubKeyList<'a> {
        self.source.collect_keys(&self.key)
    }
}

/// Configuration.
#[allow(missing_debug_implementations)]
pub struct Configuration {
    internal: LayeredSource,
}

impl Configuration {
    /// Create new configuration.
    pub fn new() -> Self {
        Self {
            internal: LayeredSource::new(),
        }
    }

    /// Register config source.
    pub fn register_source(mut self, source: impl ConfigSource + 'static) -> Self {
        self.internal.register(source);
        self
    }

    pub(crate) fn new_context(&self) -> ConfigContext<'_> {
        ConfigContext {
            key: ConfigKey::default(),
            source: self,
        }
    }

    /// Get config from configuration.
    #[inline]
    pub fn get<T: FromConfig>(&self, key: &str) -> Result<T, ConfigError> {
        let mut context = self.new_context();
        context.parse_config(key, None)
    }

    /// Get config or use default.
    #[inline]
    pub fn get_or<T: FromConfig>(&self, key: &str, def: T) -> Result<T, ConfigError> {
        Ok(self.get::<Option<T>>(key)?.unwrap_or(def))
    }

    /// Get predefined config from configuration.
    pub fn get_predefined<T: FromConfigWithPrefix>(&self) -> Result<T, ConfigError> {
        self.get(T::prefix())
    }

    /// Get source names.
    pub fn source_names(&self) -> Vec<&str> {
        self.internal.source_names()
    }

    fn collect_keys(&self, prefix: &ConfigKey<'_>) -> SubKeyList<'_> {
        let mut sub = SubKeyList::new();
        self.internal.collect_keys(prefix, &mut sub);
        sub
    }

    /// Build default configuration.
    pub fn build<F: FnOnce(MemorySource) -> Result<MemorySource, ConfigError>>(
        f: F,
    ) -> Result<Self, ConfigError> {
        let mut config = Configuration::new();

        // Layer 0, commandlines.
        config = config.register_source((f)(MemorySource::new("commandlines".to_string()))?);

        let option: SourceOption = config.get_predefined()?;

        // Layer 1, random
        #[cfg(feature = "rand")]
        if option.random.enabled {
            config = config.register_source(crate::source::random::Random);
        }

        // Layer 2, environment.
        let prefix = var("CFG_ENV_PREFIX").unwrap_or("CFG".to_owned());
        config = config.register_source(EnvironmentPrefixedSource::new(&prefix));

        // Layer 2, profile file.
        let app = config.get_predefined::<AppConfig>()?;
        if let Some(profile) = &app.profile {
            config = register_files(
                config,
                &option,
                app.dir.as_deref(),
                &app.name,
                Some(&profile),
            )?;
        }

        // Layer 3, file.
        config = register_files(config, &option, app.dir.as_deref(), &app.name, None)?;

        Ok(config)
    }
}

#[derive(Debug, FromConfig)]
#[config(prefix = "app")]
struct AppConfig {
    #[config(default = "app")]
    name: String,
    dir: Option<String>,
    profile: Option<String>,
}

#[cfg(test)]
mod test {

    use crate::source::memory::MemorySource;

    use super::*;

    macro_rules! should_eq {
        ($context:ident: $val:literal as $t:ty = $x:expr  ) => {
            println!("{} key: {}", type_name::<$t>(), $val);
            assert_eq!($x, &format!("{:?}", $context.get::<$t>($val)));
        };
    }

    fn build_config() -> Configuration {
        Configuration::new().register_source(
            MemorySource::default()
                .set("a", "0")
                .set("b", "${b}")
                .set("c", "${a}")
                .set("d", "${z}")
                .set("e", "${z:}")
                .set("f", "${z:${a}}")
                .set("g", "a")
                .set("h", "${${g}}")
                .set("i", "\\$\\{a\\}")
                .set("j", "${${g}:a}")
                .set("k", "${a} ${a}")
                .set("l", "${c}")
                .set("m", "${no_found:${no_found_2:hello}}"),
        )
    }

    #[test]
    fn parse_string_test() {
        let config = build_config();
        should_eq!(config: "a" as String = "Ok(\"0\")");
        should_eq!(config: "b" as String = "Err(ConfigRecursiveError(\"b\"))");
        should_eq!(config: "c" as String = "Ok(\"0\")");
        should_eq!(config: "d" as String = "Err(ConfigRecursiveNotFound(\"z\"))");
        should_eq!(config: "e" as String = "Ok(\"\")");
        should_eq!(config: "f" as String = "Ok(\"0\")");
        should_eq!(config: "g" as String = "Ok(\"a\")");
        should_eq!(config: "h" as String = "Ok(\"0\")");
        should_eq!(config: "i" as String = "Ok(\"${a}\")");
        should_eq!(config: "j" as String = "Ok(\"0\")");
        should_eq!(config: "k" as String = "Ok(\"0 0\")");
        should_eq!(config: "l" as String = "Ok(\"0\")");
        should_eq!(config: "m" as String = "Ok(\"hello\")");
    }

    #[test]
    fn parse_bool_test() {
        let config = build_config();
        should_eq!(config: "a" as bool = "Err(ConfigParseError(\"a\", \"0\"))");
        should_eq!(config: "b" as bool = "Err(ConfigRecursiveError(\"b\"))");
        should_eq!(config: "c" as bool = "Err(ConfigParseError(\"c\", \"0\"))");
        should_eq!(config: "d" as bool = "Err(ConfigRecursiveNotFound(\"z\"))");
        should_eq!(config: "e" as bool = "Err(ConfigNotFound(\"e\"))");
        should_eq!(config: "f" as bool = "Err(ConfigParseError(\"f\", \"0\"))");
        should_eq!(config: "g" as bool = "Err(ConfigParseError(\"g\", \"a\"))");
        should_eq!(config: "h" as bool = "Err(ConfigParseError(\"h\", \"0\"))");
        should_eq!(config: "i" as bool = "Err(ConfigParseError(\"i\", \"${a}\"))");
        should_eq!(config: "j" as bool = "Err(ConfigParseError(\"j\", \"0\"))");
        should_eq!(config: "k" as bool = "Err(ConfigParseError(\"k\", \"0 0\"))");
        should_eq!(config: "l" as bool = "Err(ConfigParseError(\"l\", \"0\"))");
        should_eq!(config: "m" as bool = "Err(ConfigParseError(\"m\", \"hello\"))");
    }

    #[test]
    fn parse_u8_test() {
        let config = build_config();
        should_eq!(config: "a" as u8 = "Ok(0)");
        should_eq!(config: "b" as u8 = "Err(ConfigRecursiveError(\"b\"))");
        should_eq!(config: "c" as u8 = "Ok(0)");
        should_eq!(config: "d" as u8 = "Err(ConfigRecursiveNotFound(\"z\"))");
        should_eq!(config: "e" as u8 = "Err(ConfigNotFound(\"e\"))");
        should_eq!(config: "f" as u8 = "Ok(0)");
        should_eq!(config: "g" as u8 = "Err(ConfigCause(ParseIntError { kind: InvalidDigit }))");
        should_eq!(config: "h" as u8 = "Ok(0)");
        should_eq!(config: "i" as u8 = "Err(ConfigCause(ParseIntError { kind: InvalidDigit }))");
        should_eq!(config: "j" as u8 = "Ok(0)");
        should_eq!(config: "k" as u8 = "Err(ConfigCause(ParseIntError { kind: InvalidDigit }))");
        should_eq!(config: "l" as u8 = "Ok(0)");
        should_eq!(config: "m" as u8 = "Err(ConfigCause(ParseIntError { kind: InvalidDigit }))");
    }
}
