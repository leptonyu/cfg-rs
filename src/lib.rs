#![doc = include_str!("../README.md")]
#![doc(issue_tracker_base_url = "https://github.com/leptonyu/cfg-rs/issues/")]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![cfg_attr(coverage_nightly, feature(coverage_attribute))]
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

#[cfg_attr(coverage_nightly, coverage(off))]
#[cfg(test)]
mod test;
#[cfg_attr(coverage_nightly, coverage(off))]
#[cfg(test)]
#[macro_use(quickcheck)]
extern crate quickcheck_macros;

mod cache;
mod configuration;
mod derive;
mod err;
mod key;

mod prelude;
pub mod source;
mod value;
mod value_ref;

use key::PartialKeyCollector;

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
pub use prelude::*;
#[allow(unused_imports)]
#[cfg(feature = "log")]
pub use value::log as _;
#[allow(unused_imports)]
#[cfg(feature = "coarsetime")]
pub use value::time as _;
pub use value::{ConfigValue, FromStrHolder, FromStringValue, FromValue};
pub use value_ref::RefValue;

#[doc(hidden)]
pub use source::cargo::Cargo;
#[doc(hidden)]
pub use source::file::inline_source_config;

use std::sync::*;

pub(crate) mod macros {
    macro_rules! cfg_log {
    ($b:expr => $lvl:expr,$($arg:tt)+) => {
        #[cfg(feature = "log")]
        {
            if  $b {
                log::log!($lvl, $($arg)+);
            }
        }
    };
    ($lvl:expr,$($arg:tt)+) => {
        #[cfg(feature = "log")]
        {
            log::log!($lvl, $($arg)+);
        }
    };
    }

    macro_rules! impl_default {
        ($x:ident) => {
            impl Default for $x {
                fn default() -> Self {
                    Self::new()
                }
            }
        };
    }

    pub(crate) use cfg_log;
    pub(crate) use impl_default;
}

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
