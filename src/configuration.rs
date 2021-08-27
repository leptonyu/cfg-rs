use std::{
    any::{type_name, Any},
    borrow::Borrow,
    cell::RefCell,
    collections::HashSet,
    env::var,
    path::PathBuf,
};

use crate::{
    err::ConfigError,
    impl_cache,
    key::{CacheString, ConfigKey, PartialKeyIter},
    source::{
        environment::PrefixEnvironment, memory::HashSource, register_by_ext, register_files,
        Loader, SourceOption,
    },
    value::ConfigValue,
    FromConfig, FromConfigWithPrefix, PartialKeyCollector,
};

/// Configuration Context.
///
/// Configuration context contains current level of config key and configuration instance.
/// It is designed for parsing partial config by partial key and default value.
#[allow(missing_debug_implementations)]
pub struct ConfigContext<'a> {
    key: ConfigKey<'a>,
    source: &'a HashSource,
}

struct CacheValue {
    buf: String,
    stack: Vec<usize>,
}

impl CacheValue {
    fn new() -> Self {
        Self {
            buf: String::with_capacity(10),
            stack: Vec::with_capacity(3),
        }
    }

    fn clear(&mut self) {
        self.buf.clear();
        self.stack.clear();
    }
}

impl_cache!(CacheValue);

impl HashSource {
    fn new_context<'a>(&'a self, cache: &'a mut CacheString) -> ConfigContext<'a> {
        ConfigContext {
            key: cache.new_key(),
            source: self,
        }
    }
}

impl<'a> ConfigContext<'a> {
    fn parse_placeholder(
        source: &'a HashSource,
        current_key: &ConfigKey<'_>,
        val: &str,
        history: &mut HashSet<String>,
    ) -> Result<(bool, Option<ConfigValue<'a>>), ConfigError> {
        CacheValue::with_key(move |cv| {
            cv.clear();
            let mut value = val;
            let pat: &[_] = &['$', '\\', '}'];
            let mut flag = true;
            while let Some(pos) = value.find(pat) {
                flag = false;
                match &value[pos..=pos] {
                    "$" => {
                        let pos_1 = pos + 1;
                        if value.len() == pos_1 || &value[pos_1..=pos_1] != "{" {
                            return Err(ConfigError::ConfigRecursiveError(current_key.to_string()));
                        }
                        cv.buf.push_str(&value[..pos]);
                        cv.stack.push(cv.buf.len());
                        value = &value[pos + 2..];
                    }
                    "\\" => {
                        let pos_1 = pos + 1;
                        if value.len() == pos_1 {
                            return Err(ConfigError::ConfigRecursiveError(current_key.to_string()));
                        }
                        cv.buf.push_str(&value[..pos]);
                        cv.buf.push_str(&value[pos_1..=pos_1]);
                        value = &value[pos + 2..];
                    }
                    "}" => {
                        let last = match cv.stack.pop() {
                            Some(last) => last,
                            _ => {
                                return Err(ConfigError::ConfigParseError(
                                    current_key.to_string(),
                                    value.to_owned(),
                                ))
                            }
                        };
                        cv.buf.push_str(&value[..pos]);
                        let v = &(cv.buf.as_str())[last..];
                        let (key, def) = match v.find(':') {
                            Some(pos) => (&v[..pos], Some(&v[pos + 1..])),
                            _ => (&v[..], None),
                        };
                        if !history.insert(key.to_string()) {
                            return Err(ConfigError::ConfigRecursiveError(current_key.to_string()));
                        }
                        let v = match CacheString::with_key_place(|cache| {
                            source
                                .new_context(cache)
                                .do_parse_config::<String, &str>(key, None, history)
                        }) {
                            Err(ConfigError::ConfigNotFound(v)) => match def {
                                Some(v) => v.to_owned(),
                                _ => return Err(ConfigError::ConfigRecursiveNotFound(v)),
                            },
                            ret => ret?,
                        };
                        history.remove(key);
                        cv.buf.truncate(last);
                        cv.buf.push_str(&v);
                        value = &value[pos + 1..];
                    }
                    _ => return Err(ConfigError::ConfigRecursiveError(current_key.to_string())),
                }
            }
            if flag {
                return Ok((true, None));
            }
            if cv.stack.pop().unwrap_or(0) == 0 {
                if cv.stack.is_empty() {
                    return Ok((false, Some(cv.buf.to_string().into())));
                }
            }
            Ok((false, None))
        })
    }

    #[inline]
    pub(crate) fn do_parse_config<T: FromConfig, K: Into<PartialKeyIter<'a>>>(
        &mut self,
        partial_key: K,
        default_value: Option<ConfigValue<'_>>,
        history: &mut HashSet<String>,
    ) -> Result<T, ConfigError> {
        self.key.push(partial_key);
        let value = match self.source.get_value(&self.key).or(default_value) {
            Some(ConfigValue::Str(s)) => {
                match Self::parse_placeholder(self.source, &self.key, &s, history)? {
                    (true, _) => Some(ConfigValue::Str(s)),
                    (_, v) => v,
                }
            }
            Some(ConfigValue::StrRef(s)) => {
                match Self::parse_placeholder(self.source, &self.key, s, history)? {
                    (true, _) => Some(ConfigValue::StrRef(s)),
                    (false, v) => v,
                }
            }
            #[cfg(feature = "rand")]
            Some(ConfigValue::Rand(s)) => Some(ConfigValue::normalize(s)),
            v => v,
        };

        let v = T::from_config(self, value);
        self.key.pop();
        v
    }

    /// Parse partial config by partial key and default value.
    #[inline]
    pub fn parse_config<T: FromConfig>(
        &mut self,
        partial_key: &'a str,
        default_value: Option<ConfigValue<'_>>,
    ) -> Result<T, ConfigError> {
        self.do_parse_config(partial_key, default_value, &mut HashSet::new())
    }

    /// Get current key in context.
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
            #[cfg(feature = "rand")]
            ConfigValue::Rand(_) => "Random",
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

    pub(crate) fn collect_keys(&self) -> PartialKeyCollector<'a> {
        let mut c = PartialKeyCollector::new();
        self.source.collect_keys(&self.key, &mut c);
        c
    }
}

/// Configuration Instance.
///
/// Configuration instance simply contains a multi layered [`ConfigSource`].
#[allow(missing_debug_implementations)]
pub struct Configuration {
    source: HashSource,
    loaders: Vec<Box<dyn Loader + 'static>>,
}

/// Configuration Builder.
///
/// Configuration builder is used for customizing some configs by programming.
#[allow(missing_debug_implementations)]
pub struct ConfigurationBuilder {
    memory: HashSource,
    prefix: Option<String>,
}

impl Configuration {
    /// Create new empty configuration. Normally you don't need this.
    /// Try [`Configuration::init`] or [`Configuration::builder`].
    pub fn new() -> Self {
        Self {
            source: HashSource::new("configuration"),
            loaders: vec![],
        }
    }

    pub(crate) fn new_context<'a>(&'a self, cache: &'a mut CacheString) -> ConfigContext<'a> {
        ConfigContext {
            key: cache.new_key(),
            source: &self.source,
        }
    }

    /// Register prefix env.
    pub fn register_perfix_env(&mut self, prefix: &str) -> Result<&mut Self, ConfigError> {
        self.register_loader(PrefixEnvironment::new(prefix))
    }

    /// Register file source, please enable format feature.
    pub fn register_file(
        &mut self,
        path: PathBuf,
        required: bool,
    ) -> Result<&mut Self, ConfigError> {
        register_by_ext(self, path, required)?;
        Ok(self)
    }

    /// Register random source, which can get random values.
    #[cfg(feature = "rand")]
    #[cfg_attr(docsrs, doc(cfg(feature = "rand")))]
    pub fn register_random(&mut self) -> Result<&mut Self, ConfigError> {
        self.register_loader(crate::source::random::Random)
    }

    /// Register source loader.
    pub fn register_loader<L: Loader + 'static>(
        &mut self,
        loader: L,
    ) -> Result<&mut Self, ConfigError> {
        loader.load(&mut self.source.prefixed())?;
        self.loaders.push(Box::new(loader));
        Ok(self)
    }

    /// Get config from configuration by key.
    /// Please refer to [`ConfigKey`] for the key's pattern details.
    #[inline]
    pub fn get<T: FromConfig>(&self, key: &str) -> Result<T, ConfigError> {
        CacheString::with_key(|cache| {
            let mut context = self.new_context(cache);
            context.parse_config(key, None)
        })
    }

    /// Get config from configuration by key, otherwise return default.
    /// Please refer to [`ConfigKey`] for the key's pattern details.
    #[inline]
    pub fn get_or<T: FromConfig>(&self, key: &str, def: T) -> Result<T, ConfigError> {
        Ok(self.get::<Option<T>>(key)?.unwrap_or(def))
    }

    /// Get config with predefined key, which is automatically derived by [FromConfig](./derive.FromConfig.html#struct-annotation-attribute).
    pub fn get_predefined<T: FromConfigWithPrefix>(&self) -> Result<T, ConfigError> {
        self.get(T::prefix())
    }

    /// Get source names, just for test.
    pub fn source_names(&self) -> Vec<&str> {
        self.loaders.iter().map(|l| l.name()).collect()
    }

    /// Create a configuration builder to customize the configuration instance.
    pub fn builder() -> ConfigurationBuilder {
        ConfigurationBuilder {
            memory: HashSource::new("fixed:FromProgram/CommandLineArgs"),
            prefix: None,
        }
    }

    /// Create configuration instance with default settings.
    /// Please refer to [`ConfigurationBuilder::init`] for details.
    pub fn init() -> Result<Self, ConfigError> {
        Self::builder().init()
    }
}

impl ConfigurationBuilder {
    /// Set environment prefix, default value if `CFG`.
    /// By default, following environment variables will be loaded in to configuration.
    /// # Environment Variable Regex Pattern: `CFG_[0-9a-zA-Z]+`
    ///
    /// These are some predefined environment variables:
    /// * `CFG_ENV_PREFIX=CFG`
    /// * `CFG_APP_NAME=app`
    /// * `CFG_APP_DIR=`
    /// * `CFG_APP_PROFILE=`
    ///
    /// You can change `CFG` to other prefix by this method.
    pub fn set_env_prefix<K: ToString>(&mut self, prefix: K) -> &mut Self {
        self.prefix = Some(prefix.to_string());
        self
    }

    /// Set config into configuration by programming.
    /// You can use this method to set default values.
    pub fn set<K: Borrow<str>, V: Into<ConfigValue<'static>>>(mut self, key: K, value: V) -> Self {
        self.memory = self.memory.set(key, value);
        self
    }

    /// Initialize configuration by multiple predefined sources.
    ///
    /// ## Predefined Sources.
    ///
    /// 1. [x] Customized by Programming
    /// 2. [ ] Random Value
    /// 3. [x] Environment Variable with Prefix `CFG`, referto [`ConfigurationBuilder::set_env_prefix`] for details.
    /// 4. [ ] Profiled File Source with Path, `${app.dir}/${app.name}-${app.profile}.EXT`. EXT: toml, json, yaml.
    /// 5. [ ] File Source with Path, `${app.dir}/${app.name}.EXT`. EXT: toml, json, yaml.
    /// 6. [ ] Customized Source Can be Registered by [`Configuration::register_source`].
    ///
    /// ## Crate Feature
    ///
    /// * Feature `rand` to enable random value source.
    /// * Feature `toml` to enable toml supports.
    /// * Feature `yaml` to enable yaml supports.
    /// * Feature `json` to enable json supports.
    pub fn init(self) -> Result<Configuration, ConfigError> {
        let mut config = Configuration::new();

        // Layer 0, commandlines.
        config.register_loader(self.memory)?;

        let option: SourceOption = config.get_predefined()?;

        // Layer 1, random
        #[cfg(feature = "rand")]
        if option.random.enabled {
            config.register_loader(crate::source::random::Random)?;
        }

        // Layer 2, environment.
        let prefix = self
            .prefix
            .or_else(|| config.get::<Option<String>>("env.prefix").ok().flatten())
            .or_else(|| var("CFG_ENV_PREFIX").ok())
            .unwrap_or("CFG".to_owned());
        config.register_loader(PrefixEnvironment::new(&prefix))?;

        // Layer 2, profile file.
        let app = config.get_predefined::<AppConfig>()?;
        let mut path = PathBuf::new();
        if let Some(d) = app.dir {
            path.push(d);
        };
        if let Some(profile) = &app.profile {
            let mut path = path.clone();
            path.push(format!("{}-{}", app.name, profile));
            register_files(&mut config, &option, path, false)?;
        }

        // Layer 3, file.
        path.push(app.name);
        register_files(&mut config, &option, path, false)?;

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

    use crate::test::TestConfigExt;

    use super::*;

    macro_rules! should_eq {
        ($context:ident: $val:literal as $t:ty = $x:expr  ) => {
            println!("{} key: {}", type_name::<$t>(), $val);
            assert_eq!($x, &format!("{:?}", $context.get::<$t>($val)));
        };
    }

    fn build_config() -> Configuration {
        HashSource::new("test")
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
            .set("m", "${no_found:${no_found_2:hello}}")
            .new_config()
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
