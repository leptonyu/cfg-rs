//! Configuration sources module, use it when you want to extend config sources.
use crate::*;

#[allow(unused_imports)]
use self::file::FileSource;

/// Config key module.
pub mod key {
    pub use crate::key::{CacheKey, PartialKey, PartialKeyCollector};
}

pub mod environment;
pub mod file;
#[doc(hidden)]
#[cfg(feature = "json")]
#[cfg_attr(docsrs, doc(cfg(feature = "json")))]
pub mod json;
pub mod layered;
pub mod memory;
#[doc(hidden)]
#[cfg(feature = "rand")]
#[cfg_attr(docsrs, doc(cfg(feature = "rand")))]
pub mod random;
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

#[allow(unused_mut, unused_variables)]
pub(crate) fn register_files(
    mut config: Configuration,
    option: &SourceOption,
    dir: Option<&str>,
    name: &str,
    profile: Option<&str>,
) -> Result<Configuration, ConfigError> {
    let dir = dir.unwrap_or("");
    #[cfg(feature = "toml")]
    if option.toml.enabled {
        let source: FileSource<toml::Toml> = FileSource::of(dir, name, profile)?;
        config = config.register_source(source);
    }
    #[cfg(feature = "yaml")]
    if option.yaml.enabled {
        let source: FileSource<yaml::Yaml> = FileSource::of(dir, name, profile)?;
        config = config.register_source(source);
    }
    #[cfg(feature = "json")]
    if option.json.enabled {
        let source: FileSource<json::Json> = FileSource::of(dir, name, profile)?;
        config = config.register_source(source);
    }
    Ok(config)
}
