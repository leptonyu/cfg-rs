//! cfg-rs.

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

mod configuration;
mod derive;
mod err;
mod key;
pub mod source;
#[cfg(test)]
mod test;
mod value;

pub use cfg_derive::FromConfig;
pub use configuration::{ConfigContext, Configuration};
pub use derive::FromConfigWithPrefix;
pub use err::ConfigError;
pub use key::{ConfigKey, SubKeyList};
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
    fn collect_keys<'a>(&'a self, prefix: &ConfigKey<'_>, sub: &mut SubKeyList<'a>);

    /// Is empty.
    fn is_empty(&self) -> bool;
}
