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
        cargo::Cargo, environment::PrefixEnvironment, memory::HashSource, register_by_ext,
        register_files, ConfigSource, SourceOption,
    },
    value::ConfigValue,
    value_ref::Refresher,
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
    pub(crate) ref_value_flag: bool,
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
    pub(crate) fn new_context<'a>(&'a self, cache: &'a mut CacheString) -> ConfigContext<'a> {
        ConfigContext {
            key: cache.new_key(),
            source: self,
            ref_value_flag: false,
        }
    }
}

impl<'a> ConfigContext<'a> {
    pub(crate) fn as_refresher(&self) -> &Refresher {
        &self.source.refs
    }

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
                            return Err(ConfigError::ConfigParseError(
                                current_key.to_string(),
                                val.to_owned(),
                            ));
                        }
                        cv.buf.push_str(&value[..pos]);
                        cv.stack.push(cv.buf.len());
                        value = &value[pos + 2..];
                    }
                    "\\" => {
                        let pos_1 = pos + 1;
                        if value.len() == pos_1 {
                            return Err(ConfigError::ConfigParseError(
                                current_key.to_string(),
                                val.to_owned(),
                            ));
                        }
                        cv.buf.push_str(&value[..pos]);
                        cv.buf.push_str(&value[pos_1..=pos_1]);
                        value = &value[pos + 2..];
                    }
                    "}" => {
                        let last = cv.stack.pop().ok_or_else(|| {
                            ConfigError::ConfigParseError(current_key.to_string(), val.to_owned())
                        })?;

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

            if cv.stack.is_empty() {
                return Ok((false, Some(cv.buf.to_string().into())));
            }

            Err(ConfigError::ConfigParseError(
                current_key.to_string(),
                val.to_owned(),
            ))
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
            Some(ConfigValue::StrRef(s)) => {
                match Self::parse_placeholder(self.source, &self.key, s, history)? {
                    (true, _) => Some(ConfigValue::StrRef(s)),
                    (false, v) => v,
                }
            }
            Some(ConfigValue::Str(s)) => {
                match Self::parse_placeholder(self.source, &self.key, &s, history)? {
                    (true, _) => Some(ConfigValue::Str(s)),
                    (_, v) => v,
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

/// Configuration Instance, See [Examples](https://github.com/leptonyu/cfg-rs/tree/main/examples),
/// [How to Initialize Configuration](index.html#how-to-initialize-configuration) for details.
#[allow(missing_debug_implementations)]
pub struct Configuration {
    pub(crate) source: HashSource,
    loaders: Vec<Box<dyn ConfigSource + Send + 'static>>,
}

impl Configuration {
    /// Create an empty [`Configuration`].
    ///
    /// If you want to use predefined sources, please try [`Configuration::with_predefined`] or [`Configuration::with_predefined_builder`].
    ///
    pub fn new() -> Self {
        Self {
            source: HashSource::new("configuration"),
            loaders: vec![],
        }
    }

    /// Register key value manually.
    pub fn register_kv<N: Into<String>>(self, name: N) -> ManualSource {
        ManualSource(self, HashSource::new(name))
    }

    /// Register all env variables with prefix, default prefix is `CFG`.
    ///
    /// * `prefix` - Env variable prefix.
    ///
    /// If prefix is `CFG`, then all env variables with pattern `CFG_*` will be added into configuration.
    ///
    /// Examples:
    /// 1. `CFG_APP_NAME` => `app.name`
    /// 2. `CFG_APP_0_NAME` => `app[0].name`
    ///
    pub fn register_prefix_env(self, prefix: &str) -> Result<Self, ConfigError> {
        self.register_source(PrefixEnvironment::new(prefix))
    }

    /// Register file source, this method uses file extension[^ext] to choose how to parsing configuration.
    ///
    /// * `path` - Config file path.
    /// * `required` - Whether config file must exist.
    ///
    /// See [Supported File Formats](index.html#supported-file-format) for details.
    ///
    /// [^ext]: `cfg-rs` does not **enable** any file format by default, please enable specific features when use this method.
    pub fn register_file<P: Into<PathBuf>>(
        self,
        path: P,
        required: bool,
    ) -> Result<Self, ConfigError> {
        register_by_ext(self, path.into(), required)
    }

    /// Register random value source, must enable feature **rand**.
    ///
    /// Supported integer types:
    /// * random.u8
    /// * random.u16
    /// * random.u32
    /// * random.u64
    /// * random.u128
    /// * random.usize
    /// * random.i8
    /// * random.i16
    /// * random.i32
    /// * random.i64
    /// * random.i128
    /// * random.isize
    #[cfg(feature = "rand")]
    #[cfg_attr(docsrs, doc(cfg(feature = "rand")))]
    pub fn register_random(self) -> Result<Self, ConfigError> {
        self.register_source(crate::source::random::Random)
    }

    /// Register customized source, see [How to Initialize Configuration](index.html#how-to-initialize-configuration),
    /// [ConfigSource](source/trait.ConfigSource.html) for details.
    pub fn register_source<L: ConfigSource + 'static>(
        mut self,
        loader: L,
    ) -> Result<Self, ConfigError> {
        loader.load(&mut self.source.prefixed())?;
        self.loaders.push(Box::new(loader));
        Ok(self)
    }

    #[inline]
    fn reload(&self) -> Result<Configuration, ConfigError> {
        let mut s = Configuration::new();
        let c = &mut s.source.prefixed();
        for l in self.loaders.iter() {
            l.load(c)?;
        }
        self.source.refs.refresh(&s)?;
        Ok(s)
    }

    /// Refresh all [RefValue](struct.RefValue.html)s without change [`Configuration`] itself.
    pub fn refresh_ref(&self) -> Result<(), ConfigError> {
        let _ = self.reload()?;
        Ok(())
    }

    /// Refresh all [RefValue](struct.RefValue.html)s and [`Configuration`] itself.
    pub fn refresh(&mut self) -> Result<(), ConfigError> {
        let c = self.reload()?;
        self.source.value = c.source.value;
        Ok(())
    }

    /// Get config from configuration by key, see [`ConfigKey`] for the key's pattern details.
    ///
    /// * `key` - Config Key.
    /// Key examples:
    /// 1. `cfg.v1`
    /// 2. `cfg.v2[0]`
    /// 3. `cfg.v3[0][1]`
    /// 4. `cfg.v4.key`
    /// 5. `cfg.v5.arr[0]`
    #[inline]
    pub fn get<T: FromConfig>(&self, key: &str) -> Result<T, ConfigError> {
        CacheString::with_key(|cache| {
            let mut context = self.source.new_context(cache);
            context.parse_config(key, None)
        })
    }

    /// Get config from configuration by key, otherwise return default. See [`ConfigKey`] for the key's pattern details.
    ///
    /// * `key` - Config Key.
    /// * `def` - If config value is not found, then return def.
    #[inline]
    pub fn get_or<T: FromConfig>(&self, key: &str, def: T) -> Result<T, ConfigError> {
        Ok(self.get::<Option<T>>(key)?.unwrap_or(def))
    }

    /// Get config with predefined key, which is automatically derived by [FromConfig](./derive.FromConfig.html#struct-annotation-attribute).
    #[inline]
    pub fn get_predefined<T: FromConfigWithPrefix>(&self) -> Result<T, ConfigError> {
        self.get(T::prefix())
    }

    /// Get source names, just for test.
    pub fn source_names(&self) -> Vec<&str> {
        self.loaders.iter().map(|l| l.name()).collect()
    }

    /// Create predefined sources builder, see [init](struct.PredefinedConfigurationBuilder.html#method.init) for details.
    pub fn with_predefined_builder() -> PredefinedConfigurationBuilder {
        PredefinedConfigurationBuilder {
            memory: HashSource::new("fixed:FromProgram/CommandLineArgs"),
            cargo: None,
            prefix: None,
        }
    }

    /// Create predefined configuration, see [init](struct.PredefinedConfigurationBuilder.html#method.init) for details.
    pub fn with_predefined() -> Result<Self, ConfigError> {
        Self::with_predefined_builder().init()
    }
}

/// Predefined Configuration Builder. See [init](struct.PredefinedConfigurationBuilder.html#method.init) for details.
#[allow(missing_debug_implementations)]
pub struct PredefinedConfigurationBuilder {
    memory: HashSource,
    cargo: Option<Cargo>,
    prefix: Option<String>,
}

impl PredefinedConfigurationBuilder {
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
    pub fn set_prefix_env<K: ToString>(&mut self, prefix: K) -> &mut Self {
        self.prefix = Some(prefix.to_string());
        self
    }

    /// Set config into configuration by programming, or from command line arguments.
    pub fn set<K: Borrow<str>, V: Into<ConfigValue<'static>>>(mut self, key: K, value: V) -> Self {
        self.memory = self.memory.set(key, value);
        self
    }

    /// Set source from cargo build env. Macro [init_cargo_env](macro.init_cargo_env.html) will collect
    /// all `CARGO_PKG_*` env variables, and `CARGO_BIN_NAME` into configuration.
    ///
    /// ### Usage
    /// ```rust, no_run
    /// use cfg_rs::*;
    /// // Generate fn init_cargo_env().
    /// init_cargo_env!();
    /// let c = Configuration::with_predefined_builder()
    ///   .set_cargo_env(init_cargo_env())
    ///   .init()
    ///   .unwrap();
    /// ```
    pub fn set_cargo_env(mut self, cargo: Cargo) -> Self {
        self.cargo = Some(cargo);
        self
    }

    /// Initialize configuration by multiple predefined sources.
    ///
    /// ## Predefined Sources.
    ///
    /// 0. Cargo Package Env Variables (Must be explicitly set by [set_cargo_env](struct.PredefinedConfigurationBuilder.html#method.set_cargo_env)).
    /// 1. Customized by Programming or Commandline Args.[^f_default]
    /// 2. Random Value (Auto enabled with feature `rand`).
    /// 3. Environment Variable with Prefix `CFG`, referto [set_prefix_env](struct.PredefinedConfigurationBuilder.html#method.set_prefix_env) for details.[^f_default]
    /// 4. Profiled File Source with Path, `${app.dir}/${app.name}-${app.profile}.EXT`. EXT: toml, json, yaml.[^f_file]
    /// 5. File Source with Path, `${app.dir}/${app.name}.EXT`. EXT: toml, json, yaml.[^f_file]
    /// 6. Customized Source Can be Registered by [register_source](struct.Configuration.html#method.register_source).
    ///
    /// [^f_default]: Always be enabled.
    ///
    /// [^f_file]: See [Supported File Formats](index.html#supported-file-format) for details.
    ///
    /// ## Crate Feature
    ///
    /// * Feature `rand` to enable random value source.
    /// * Feature `toml` to enable toml supports.
    /// * Feature `yaml` to enable yaml supports.
    /// * Feature `json` to enable json supports.
    pub fn init(self) -> Result<Configuration, ConfigError> {
        let mut config = Configuration::new();

        // Layer 0, cargo dev envs.
        if let Some(cargo) = self.cargo {
            config = config.register_source(cargo)?;
        }

        // Layer 1, commandlines.
        config = config.register_source(self.memory)?;

        let option: SourceOption = config.get_predefined()?;

        // Layer 2, random
        #[cfg(feature = "rand")]
        if option.random.enabled {
            config = config.register_random()?;
        }

        // Layer 3, environment.
        let prefix = self
            .prefix
            .or_else(|| config.get::<Option<String>>("env.prefix").ok().flatten())
            .or_else(|| var("CFG_ENV_PREFIX").ok())
            .unwrap_or("CFG".to_owned());
        config = config.register_prefix_env(&prefix)?;

        // Layer 4, profile file.
        let app = config.get_predefined::<AppConfig>()?;
        let mut path = PathBuf::new();
        if let Some(d) = app.dir {
            path.push(d);
        };
        if let Some(profile) = &app.profile {
            let mut path = path.clone();
            path.push(format!("{}-{}", app.name, profile));
            config = register_files(config, &option, path, false)?;
        }

        // Layer 5, file.
        path.push(app.name);
        config = register_files(config, &option, path, false)?;

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

/// Manually register key value to [`Configuration`].
#[allow(missing_debug_implementations)]
pub struct ManualSource(Configuration, HashSource);

impl ManualSource {
    /// Set config into configuration by programming, or from command line arguments.
    pub fn set<K: Borrow<str>, V: Into<ConfigValue<'static>>>(mut self, key: K, value: V) -> Self {
        self.0.source = self.0.source.set(key, value);
        self
    }

    /// Finish customized kv.
    pub fn finish(self) -> Result<Configuration, ConfigError> {
        self.0.register_source(self.1)
    }
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
            .set("n", "$")
            .set("o", "\\")
            .set("p", "}")
            .set("q", "${")
            .new_config()
            .register_kv("test")
            .set("a0", "0")
            .set("a", "1")
            .set("b", "1")
            .set("c", "1")
            .finish()
            .unwrap()
    }

    #[test]
    fn parse_string_test() {
        let config = build_config();
        should_eq!(config: "a0" as String = "Ok(\"0\")");

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
        should_eq!(config: "n" as String = "Err(ConfigParseError(\"n\", \"$\"))");
        should_eq!(config: "o" as String = "Err(ConfigParseError(\"o\", \"\\\\\"))");
        should_eq!(config: "p" as String = "Err(ConfigParseError(\"p\", \"}\"))");
        should_eq!(config: "q" as String = "Err(ConfigParseError(\"q\", \"${\"))");
    }

    #[test]
    fn parse_bool_test() {
        let config = build_config();
        should_eq!(config: "a0" as bool = "Err(ConfigParseError(\"a0\", \"0\"))");

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
        should_eq!(config: "n" as bool = "Err(ConfigParseError(\"n\", \"$\"))");
        should_eq!(config: "o" as bool = "Err(ConfigParseError(\"o\", \"\\\\\"))");
        should_eq!(config: "p" as bool = "Err(ConfigParseError(\"p\", \"}\"))");
        should_eq!(config: "q" as bool = "Err(ConfigParseError(\"q\", \"${\"))");
    }

    #[test]
    fn parse_u8_test() {
        let config = build_config();
        should_eq!(config: "a0" as u8 = "Ok(0)");

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
        should_eq!(config: "n" as u8 = "Err(ConfigParseError(\"n\", \"$\"))");
        should_eq!(config: "o" as u8 = "Err(ConfigParseError(\"o\", \"\\\\\"))");
        should_eq!(config: "p" as u8 = "Err(ConfigParseError(\"p\", \"}\"))");
        should_eq!(config: "q" as u8 = "Err(ConfigParseError(\"q\", \"${\"))");
    }

    #[test]
    fn predefined_test() {
        let _config = Configuration::with_predefined().unwrap();
        let _conf2 = Configuration::with_predefined_builder().init().unwrap();
        println!("Total count = {}", _conf2.source.value.len());
        for v in _config.source_names() {
            println!("{}", v);
        }
        assert_eq!(_conf2.source.value.len(), _config.source.value.len());
    }
}
