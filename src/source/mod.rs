//! UNSTABLE: Configuration sources module, use it when you want to extend config sources.
use crate::*;

#[allow(unused_imports)]
use self::file::FileLoader;
use std::path::PathBuf;

/// Config key module.
pub mod key {
    pub use crate::key::{CacheKey, PartialKey, PartialKeyCollector};
}
pub use file::inline_source;
pub use memory::ConfigSourceBuilder;

pub(crate) mod environment;
pub(crate) mod file;
#[doc(hidden)]
#[cfg(feature = "json")]
#[cfg_attr(docsrs, doc(cfg(feature = "json")))]
pub mod json;
pub(crate) mod memory;
#[doc(hidden)]
#[cfg(feature = "rand")]
#[cfg_attr(docsrs, doc(cfg(feature = "rand")))]
pub(crate) mod random;
#[doc(hidden)]
#[cfg(feature = "toml")]
#[cfg_attr(docsrs, doc(cfg(feature = "toml")))]
pub mod toml;
#[doc(hidden)]
#[cfg(feature = "yaml")]
#[cfg_attr(docsrs, doc(cfg(feature = "yaml")))]
pub mod yaml;

#[derive(Debug, FromConfig)]
pub(crate) struct EnabledOption {
    #[config(default = true)]
    pub(crate) enabled: bool,
}

#[derive(Debug, FromConfig)]
#[config(prefix = "app.sources")]
pub(crate) struct SourceOption {
    #[cfg(feature = "rand")]
    pub(crate) random: EnabledOption,
    #[cfg(feature = "toml")]
    toml: EnabledOption,
    #[cfg(feature = "yaml")]
    yaml: EnabledOption,
    #[cfg(feature = "json")]
    json: EnabledOption,
}

#[allow(unreachable_code, unused_variables)]
pub(crate) fn register_by_ext(
    config: &mut Configuration,
    path: PathBuf,
    required: bool,
) -> Result<(), ConfigError> {
    let ext = path
        .extension()
        .and_then(|x| x.to_str())
        .ok_or_else(|| ConfigError::ConfigFileNotSupported(path.clone()))?;
    match ext {
        #[cfg(feature = "toml")]
        "toml" => {
            config.register_source(<FileLoader<toml::Toml>>::new(path.clone(), required, true))?;
        }
        #[cfg(feature = "yaml")]
        "yaml" | "yml" => {
            config.register_source(<FileLoader<yaml::Yaml>>::new(path.clone(), required, true))?;
        }
        #[cfg(feature = "json")]
        "json" => {
            config.register_source(<FileLoader<json::Json>>::new(path.clone(), required, true))?;
        }
        _ => return Err(ConfigError::ConfigFileNotSupported(path)),
    }
    Ok(())
}

#[allow(unused_mut, unused_variables)]
pub(crate) fn register_files(
    config: &mut Configuration,
    option: &SourceOption,
    path: PathBuf,
    has_ext: bool,
) -> Result<(), ConfigError> {
    #[cfg(feature = "toml")]
    if option.toml.enabled {
        config.register_source(<FileLoader<toml::Toml>>::new(path.clone(), false, has_ext))?;
    }
    #[cfg(feature = "yaml")]
    if option.yaml.enabled {
        config.register_source(<FileLoader<yaml::Yaml>>::new(path.clone(), false, has_ext))?;
    }
    #[cfg(feature = "json")]
    if option.json.enabled {
        config.register_source(<FileLoader<json::Json>>::new(path.clone(), false, has_ext))?;
    }
    Ok(())
}

/// Source adaptor, usually convert intermediate representation config.
pub trait ConfigSourceAdaptor {
    /// Read source.
    fn convert_source(self, builder: &mut ConfigSourceBuilder<'_>) -> Result<(), ConfigError>;
}

/// Parse source intermediate representation from string.
pub trait ConfigSourceParser {
    /// Source Loader.
    type Adaptor: ConfigSourceAdaptor;

    /// Create loader.
    fn parse_source(_: &str) -> Result<Self::Adaptor, ConfigError>;

    /// File extenstions.
    fn file_extensions() -> Vec<&'static str>;
}

/// Config source.
pub trait ConfigSource {
    /// Config source name.
    fn name(&self) -> &str;

    /// Load config source.
    fn load(&self, builder: &mut ConfigSourceBuilder<'_>) -> Result<(), ConfigError>;
}
