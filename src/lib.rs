//! Cfg-rs provides a layered configuration formed by multi config source for rust applications.
//!
//! This lib supports:
//! * Mutiple sources, such as environment variables, toml, yaml and json.
//! * Easily extends config source by implementing [`crate::source::file::FileConfigSource`].
//! * Programmatic override config by [`ConfigurationBuilder::set`].
//! * Auto derive config struct by proc-macro.
//! * Placeholder parsing with syntax `${config.key}`.
//! * Using placeholder expresion to get random value by `${random.u64}`, support all integer types.
//!
//! See the [examples](https://github.com/leptonyu/cfg-rs/tree/master/examples) for general usage information.
//!

#![cfg_attr(docsrs, feature(doc_cfg))]
#![warn(
    anonymous_parameters,
    missing_copy_implementations,
    missing_debug_implementations,
    missing_docs,
    nonstandard_style,
    rust_2018_idioms,
    single_use_lifetimes,
    trivial_casts,
    trivial_numeric_casts,
    unreachable_pub,
    unused_extern_crates,
    unused_qualifications,
    variant_size_differences
)]

#[cfg(test)]
#[macro_use(quickcheck)]
extern crate quickcheck_macros;

mod cache;
mod configuration;
mod derive;
mod err;
mod key;

pub mod source;
#[cfg(test)]
mod test;
mod value;

use key::PartialKeyCollector;

/// Automatic derive [`FromConfig`] instance.
pub use cfg_derive::FromConfig;
pub use configuration::{ConfigContext, Configuration, ConfigurationBuilder};
pub use derive::FromConfigWithPrefix;
pub use err::ConfigError;
pub use key::ConfigKey;
pub use value::ConfigValue;

/// Generate config instance from configuration.
pub trait FromConfig: Sized {
    /// Generate config.
    fn from_config(
        context: &mut ConfigContext<'_>,
        value: Option<ConfigValue<'_>>,
    ) -> Result<Self, ConfigError>;
}

/// Configuration source.
pub trait ConfigSource: Send + Sync {
    /// Source name.
    fn name(&self) -> &str;

    /// Get config value by key.
    fn get_value(&self, key: &ConfigKey<'_>) -> Option<ConfigValue<'_>>;

    /// Get all sub keys by prefix.
    fn collect_keys<'a>(&'a self, prefix: &ConfigKey<'_>, sub: &mut PartialKeyCollector<'a>);

    /// Is empty.
    fn is_empty(&self) -> bool;
}
