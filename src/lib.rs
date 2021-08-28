#![doc = include_str!("../README.md")]
#![doc(issue_tracker_base_url = "https://github.com/leptonyu/cfg-rs/issues/")]
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
mod value_ref;

use key::PartialKeyCollector;
#[doc(hidden)]
pub use source::cargo::Cargo;
#[doc(hidden)]
pub use source::file::inline_source;

/// Automatic derive [`FromConfig`] instance.
///
/// We use annotation attributes to customize the derived instances' behavior.
/// All attributes in `cfg-rs` have format `#[config(key = value, key2 = value2)]`.
///
/// # Struct Annotation Attribute
///
/// * `#[config(prefix = "cfg.app")]`
///
/// This attr will lead to implement trait [`FromConfigWithPrefix`].
///
/// ```ignore,rust
/// #[derive(FromConfig)]
/// #[config(prefix = "cfg.test")]
/// struct Test {
///   //fields...   
/// }
/// ```
///
/// # Field Annotation Attribute
///
/// * `#[config(name = "val")]`
///
/// This attr will replace the default config partial key, which is name of field.
///
/// ```ignore,rust
/// #[derive(FromConfig)]
/// struct Test {
///   val: u8,
///   #[config(name = "val")]
///   other: u8, // This field `other` will use the same partial key as `val`.
/// }
/// ```
///
/// * `#[config(default = true)]`
///
/// This attr provides default value for underlying field.
///
/// ```ignore,rust
/// #[derive(FromConfig)]
/// struct Test {
///   enabled: bool, // User must provide value for this field.
///   #[config(default = true)]
///   enabled_with_default: bool, // This field has default value `true`.
/// }
/// ```
pub use cfg_derive::FromConfig;
pub use configuration::{ConfigContext, Configuration, PredefinedConfigurationBuilder};
pub use derive::FromConfigWithPrefix;
pub use err::ConfigError;
pub(crate) use err::ConfigLock;
pub use key::ConfigKey;
pub use value::ConfigValue;
pub use value_ref::RefValue;

/// Generate config instance from configuration.
///
/// The most power of this crate is automatically deriving this trait.
/// Please refer to [Derive FromConfig](./derive.FromConfig.html) for details.
pub trait FromConfig: Sized {
    /// Generate config from [`ConfigValue`] under context.
    fn from_config(
        context: &mut ConfigContext<'_>,
        value: Option<ConfigValue<'_>>,
    ) -> Result<Self, ConfigError>;
}
