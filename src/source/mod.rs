//! Configuration sources module, see the [examples](https://github.com/leptonyu/cfg-rs/tree/main/examples) for general usage information.
use crate::*;

#[allow(unused_imports)]
use self::file::FileLoader;
use std::path::PathBuf;

/// Config key module.
pub mod key {
    pub use crate::key::{CacheKey, PartialKey, PartialKeyCollector};
}
pub use super::configuration::ManualSource;
pub use memory::ConfigSourceBuilder;

pub(crate) mod cargo;
pub(crate) mod environment;
pub(crate) mod file;
pub(crate) mod memory;
#[doc(hidden)]
#[cfg(feature = "rand")]
#[cfg_attr(docsrs, doc(cfg(feature = "rand")))]
pub(crate) mod random;

#[allow(dead_code)]
#[derive(Debug, FromConfig)]
pub(crate) struct EnabledOption {
    #[config(default = true)]
    pub(crate) enabled: bool,
}

macro_rules! file_block {
    ($($nm:ident.$name:literal.$file:literal: $($k:pat)|* => $x:path,)+) => {
$(
#[doc(hidden)]
#[cfg(feature = $name)]
#[cfg_attr(docsrs, doc(cfg(feature = $name)))]
pub mod $nm;
)+

#[derive(Debug, FromConfig)]
#[config(prefix = "app.sources")]
pub(crate) struct SourceOption {
    #[cfg(feature = "rand")]
    pub(crate) random: EnabledOption,
    $(
    #[cfg(feature = $name)]
    $nm: EnabledOption,
    )+
}

#[inline]
#[allow(unreachable_code, unused_variables, unused_mut)]
pub(crate) fn register_by_ext(
    mut config: Configuration,
    path: PathBuf,
    required: bool,
) -> Result<Configuration, ConfigError> {
    let ext = path
        .extension()
        .and_then(|x| x.to_str())
        .ok_or_else(|| ConfigError::ConfigFileNotSupported(path.clone()))?;
        match ext {
            $(
                #[cfg(feature = $name)]
                $($k)|* => {
                    config = config.register_source(<FileLoader<$x>>::new(
                        path.clone(),
                        required,
                        true,
                    ))?;
                }
            )+
            _ => return Err(ConfigError::ConfigFileNotSupported(path)),
        }
    Ok(config)
}

#[allow(unused_mut, unused_variables)]
pub(crate) fn register_files(
    mut config: Configuration,
    option: &SourceOption,
    path: PathBuf,
    has_ext: bool,
) -> Result<Configuration, ConfigError> {
    $(
    #[cfg(feature = $name)]
    if option.$nm.enabled {
        config =
            config.register_source(<FileLoader<$x>>::new(path.clone(), false, has_ext))?;
    }
    )+
    Ok(config)
}


#[cfg_attr(coverage_nightly, coverage(off))]
#[cfg(test)]
mod test {
    $(
    #[test]
    #[cfg(not(feature = $name))]
    fn $nm() {
        use super::memory::HashSource;
        use crate::*;

        let _v: Result<HashSource, ConfigError> = inline_source!($file);
        match _v {
          Err(ConfigError::ConfigFileNotSupported(_)) =>{}
          _ => assert_eq!(true, false),
        }
    }
    )+
}
    };
}

file_block!(
    toml."toml"."../../app.toml" : "toml" | "tml" => toml::Toml,
    yaml."yaml"."../../app.yaml" : "yaml" | "yml" => yaml::Yaml,
    json."json"."../../app.json" : "json" => json::Json,
    ini."ini"."../../app.ini" : "ini" => ini::Ini,
);

/// Inline config file in repo, see [Supported File Formats](index.html#supported-file-format).
#[macro_export]
macro_rules! inline_source {
    ($path:literal) => {
        $crate::inline_source_internal!(
        $path:
        toml."toml": "toml" | "tml" => $crate::source::toml::Toml,
        yaml."yaml": "yaml" | "yml" => $crate::source::yaml::Yaml,
        json."json": "json" => $crate::source::json::Json,
        ini."ini": "ini" => $crate::source::ini::Ini,
        )
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! inline_source_internal {
    ($path:literal: $($nm:ident.$name:literal: $($k:pat)|* => $x:path,)+) => {
        match $path.rsplit_once(".") {
            Some((_, ext)) => {
                let _name = format!("inline:{}", $path);
                let _content = include_str!($path);
                match ext {
                    $(
                    #[cfg(feature = $name)]
                    $($k)|*  => $crate::inline_source_config::<$x>(_name, _content),
                    )+
                    _ => Err($crate::ConfigError::ConfigFileNotSupported($path.into()))
                }
            }
            _ => Err($crate::ConfigError::ConfigFileNotSupported($path.into()))
        }
    };
}

/// Config source adaptor is an intermediate representation of config source.
/// It can convert to [`ConfigSource`]. We have toml, yaml and json values implement this trait.
///
/// Config source adaptor examples:
/// * Toml format.
/// * Yaml format.
/// * Json format.
/// * Ini  format.
/// * ...
pub trait ConfigSourceAdaptor {
    /// Convert adaptor to standard config source.
    fn convert_source(self, builder: &mut ConfigSourceBuilder<'_>) -> Result<(), ConfigError>;
}

/// Parse config source from string.
pub trait ConfigSourceParser: Send {
    /// Config source adaptor.
    type Adaptor: ConfigSourceAdaptor;

    /// Parse config source.
    fn parse_source(_: &str) -> Result<Self::Adaptor, ConfigError>;

    /// File extenstions.
    fn file_extensions() -> Vec<&'static str>;
}

/// Config source.
///
/// Config source examples:
/// * Load from programming.
/// * Load from environment.
/// * Load from file.
/// * Load from network.
/// * ...
pub trait ConfigSource: Send {
    /// Config source name.
    fn name(&self) -> &str;

    /// Load config source.
    fn load(&self, builder: &mut ConfigSourceBuilder<'_>) -> Result<(), ConfigError>;

    /// If this config source can be refreshed.
    fn allow_refresh(&self) -> bool {
        false
    }

    /// Check if config source is refreshable.
    ///
    /// Implementor should notice that everytime this method is called, the refreshable state **must** be reset to **false**.
    fn refreshable(&self) -> Result<bool, ConfigError> {
        Ok(false)
    }
}
