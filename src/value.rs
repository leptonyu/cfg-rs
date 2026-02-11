#![cfg_attr(coverage_nightly, feature(coverage_attribute))]
use std::{
    any::Any,
    cmp::Ordering,
    collections::{BTreeMap, HashMap, HashSet},
    ffi::OsString,
    net::{IpAddr, Ipv4Addr, Ipv6Addr, Shutdown, SocketAddr, SocketAddrV4, SocketAddrV6},
    path::PathBuf,
    str::FromStr,
    time::Duration,
};

use crate::{ConfigContext, FromConfig, err::ConfigError};

/// Config value, [ConfigSource](source/trait.ConfigSource.html) use this value to store config properties.
///
/// # Placeholder expression
///
/// Placeholder expression use normalized string representation of [`crate::ConfigKey`], with extra brackets.
/// For example: `${cfg.k1}`.
///
/// Placeholder expression is powerful in realworld application, it has following benifits:
///
/// ## Placeholder can reduce duplicated configs, use one key config to affect multiple keys.
///
/// * `app.name` = `cfg`
/// * `app.version` = `1.0.0`
/// * `app.desc` = `Application ${app.name}, version ${app.version}`
///
/// ## Placeholder can generate configs, we can use placeholder to generate random values.
///
/// * `app.id` = `${random.u64}`
/// * `app.instance` = `${app.name}-${app.id}`
///
#[derive(Debug)]
pub enum ConfigValue<'a> {
    /// String reference, supports placeholder expression.
    StrRef(&'a str),
    /// String, supports placeholder expression.
    Str(String),
    /// Integer.
    Int(i64),
    /// Float value.
    Float(f64),
    /// Bool value.
    Bool(bool),
    #[cfg(feature = "rand")]
    /// Random value.
    Rand(RandValue),
}

#[doc(hidden)]
#[cfg(feature = "rand")]
#[derive(Debug, Clone, Copy)]
pub enum RandValue {
    U8,
    U16,
    U32,
    U64,
    U128,
    Usize,
    I8,
    I16,
    I32,
    I64,
    I128,
    Isize,
}

impl ConfigValue<'_> {
    pub(crate) fn clone_static(&self) -> ConfigValue<'static> {
        match self {
            ConfigValue::StrRef(v) => ConfigValue::Str(v.to_string()),
            ConfigValue::Str(v) => ConfigValue::Str(v.to_string()),
            ConfigValue::Int(v) => ConfigValue::Int(*v),
            ConfigValue::Float(v) => ConfigValue::Float(*v),
            ConfigValue::Bool(v) => ConfigValue::Bool(*v),
            #[cfg(feature = "rand")]
            ConfigValue::Rand(v) => ConfigValue::Rand(*v),
        }
    }
}

impl From<String> for ConfigValue<'_> {
    fn from(v: String) -> Self {
        ConfigValue::Str(v)
    }
}

impl<'a> From<&'a str> for ConfigValue<'a> {
    fn from(c: &'a str) -> Self {
        ConfigValue::StrRef(c)
    }
}

macro_rules! into_config_value_le {
    ($f:ident=$t:ident: $($x:ident),*) => {$(
        impl From<$x> for ConfigValue<'_> {
            #[allow(trivial_numeric_casts)]
            fn from(c: $x) -> Self {
                ConfigValue::$f(c as $t)
            }
        })*
    };
}

into_config_value_le!(Int = i64: u8, u16, u32, i8, i16, i32, i64);
into_config_value_le!(Float = f64: f32, f64);

macro_rules! into_config_value_u {
    ($($x:ident),*) => {$(
        impl From<$x> for ConfigValue<'_> {
            fn from(c: $x) -> Self {
                if c <= i64::MAX as $x {
                    return ConfigValue::Int(c as i64);
                }
                ConfigValue::Str(c.to_string())
            }
        })*
    };
}

into_config_value_u!(u64, u128, usize);

macro_rules! into_config_value {
    ($($x:ident),*) => {$(
        impl From<$x> for ConfigValue<'_> {
            fn from(c: $x) -> Self {
                if c <= i64::MAX as $x && c>= i64::MIN as $x {
                    return ConfigValue::Int(c as i64);
                }
                ConfigValue::Str(c.to_string())
            }
        })*
    };
}

into_config_value!(i128, isize);

impl From<bool> for ConfigValue<'_> {
    fn from(v: bool) -> Self {
        ConfigValue::Bool(v)
    }
}

#[cfg(feature = "rand")]
impl From<RandValue> for ConfigValue<'_> {
    fn from(v: RandValue) -> Self {
        ConfigValue::Rand(v)
    }
}

impl FromConfig for () {
    fn from_config(
        _: &mut ConfigContext<'_>,
        _: Option<ConfigValue<'_>>,
    ) -> Result<Self, ConfigError> {
        Ok(())
    }
}

impl<V: FromConfig> FromConfig for Result<V, ConfigError> {
    fn from_config(
        context: &mut ConfigContext<'_>,
        value: Option<ConfigValue<'_>>,
    ) -> Result<Self, ConfigError> {
        Ok(V::from_config(context, value))
    }
}

impl<V: FromConfig> FromConfig for Option<V> {
    fn from_config(
        context: &mut ConfigContext<'_>,
        value: Option<ConfigValue<'_>>,
    ) -> Result<Self, ConfigError> {
        match V::from_config(context, value) {
            Err(ConfigError::ConfigNotFound(_)) => Ok(None),
            Err(err) => Err(err),
            Ok(v) => Ok(Some(v)),
        }
    }
}

impl<V: FromConfig> FromConfig for Vec<V> {
    #[inline]
    fn from_config(
        context: &mut ConfigContext<'_>,
        _: Option<ConfigValue<'_>>,
    ) -> Result<Self, ConfigError> {
        let mut vs = vec![];
        let list = context.collect_keys();
        if let Some(v) = list.int_key {
            for i in 0..v {
                vs.push(context.do_parse_config(i, None, &mut HashSet::new())?);
            }
        }
        Ok(vs)
    }
}

macro_rules! impl_map_hash {
    // Accept forms like `HashMap::new` or `HashMap::with_hasher(Default::default())`
    ($name:ident :: $($ctor:tt)+) => {
        impl<V: FromConfig, S: std::hash::BuildHasher + Default> FromConfig for $name<String, V, S> {
            #[inline]
            fn from_config(
                context: &mut ConfigContext<'_>,
                _: Option<ConfigValue<'_>>,
            ) -> Result<Self, ConfigError> {
                let mut vs = $name:: $($ctor)+;
                let list = context.collect_keys();
                for k in list.str_key {
                    vs.insert(k.to_string(), context.parse_config(k, None)?);
                }
                Ok(vs)
            }
        }
    };
}

impl_map_hash!(HashMap::with_hasher(Default::default()));

macro_rules! impl_map {
    // Accept forms like `HashMap::new` or `HashMap::with_hasher(Default::default())`
    ($name:ident :: $($ctor:tt)+) => {
        impl<V: FromConfig> FromConfig for $name<String, V> {
            #[inline]
            fn from_config(
                context: &mut ConfigContext<'_>,
                _: Option<ConfigValue<'_>>,
            ) -> Result<Self, ConfigError> {
                let mut vs = $name:: $($ctor)+;
                let list = context.collect_keys();
                for k in list.str_key {
                    vs.insert(k.to_string(), context.parse_config(k, None)?);
                }
                Ok(vs)
            }
        }
    };
}

impl_map!(BTreeMap::new());

#[doc(hidden)]
pub trait FromValue: Sized {
    fn from_value(
        context: &mut ConfigContext<'_>,
        value: ConfigValue<'_>,
    ) -> Result<Self, ConfigError>;

    #[inline]
    fn empty_value(context: &mut ConfigContext<'_>) -> Result<Self, ConfigError> {
        Err(context.not_found())
    }
}

impl<V: FromValue> FromConfig for V {
    #[inline]
    fn from_config(
        context: &mut ConfigContext<'_>,
        value: Option<ConfigValue<'_>>,
    ) -> Result<Self, ConfigError> {
        match value {
            None => Err(context.not_found()),
            Some(ConfigValue::Str(v)) if v.is_empty() => Self::empty_value(context),
            Some(ConfigValue::StrRef("")) => Self::empty_value(context),
            Some(val) => V::from_value(context, val),
        }
    }
}

impl FromValue for String {
    #[inline]
    fn from_value(
        context: &mut ConfigContext<'_>,
        value: ConfigValue<'_>,
    ) -> Result<Self, ConfigError> {
        let v = match value {
            ConfigValue::StrRef(s) => s.to_owned(),
            ConfigValue::Str(s) => s,
            ConfigValue::Int(s) => s.to_string(),
            ConfigValue::Float(s) => check_f64(context, s)?.to_string(),
            ConfigValue::Bool(s) => s.to_string(),
            #[cfg(feature = "rand")]
            _ => return Err(context.parse_error("ConfigValueError")),
        };
        Ok(v)
    }

    #[inline]
    fn empty_value(_: &mut ConfigContext<'_>) -> Result<Self, ConfigError> {
        Ok("".to_owned())
    }
}

/// Get from string.
pub trait FromStringValue: Sized + Any {
    /// Convert from string value.
    fn from_str_value(context: &mut ConfigContext<'_>, value: &str) -> Result<Self, ConfigError>;
}

impl<V: FromStringValue> FromValue for V {
    #[inline]
    fn from_value(
        context: &mut ConfigContext<'_>,
        value: ConfigValue<'_>,
    ) -> Result<Self, ConfigError> {
        match value {
            ConfigValue::StrRef(s) => V::from_str_value(context, s),
            ConfigValue::Str(s) => V::from_str_value(context, &s),
            value => Err(context.type_mismatch::<V>(&value)),
        }
    }
}

#[inline]
fn bool_from_str_value(context: &mut ConfigContext<'_>, value: &str) -> Result<bool, ConfigError> {
    match &value.to_lowercase()[..] {
        "true" | "yes" => Ok(true),
        "false" | "no" => Ok(false),
        _ => Err(context.parse_error(value)),
    }
}

impl FromValue for bool {
    #[inline]
    fn from_value(
        context: &mut ConfigContext<'_>,
        value: ConfigValue<'_>,
    ) -> Result<Self, ConfigError> {
        match value {
            ConfigValue::StrRef(s) => bool_from_str_value(context, s),
            ConfigValue::Str(s) => bool_from_str_value(context, &s),
            ConfigValue::Bool(s) => Ok(s),
            value => Err(context.type_mismatch::<bool>(&value)),
        }
    }
}

macro_rules! impl_str_value {
    ($($x:ident),+) => {$(
impl FromStringValue for $x {
    #[inline]
    fn from_str_value(_: &mut ConfigContext<'_>, value: &str) -> Result<Self, ConfigError> {
        use std::str::FromStr;
        <$x>::from_str(value).map_err(ConfigError::from_cause)
    }
}
            )+}
}

impl_str_value!(
    Ipv4Addr,
    Ipv6Addr,
    IpAddr,
    SocketAddrV4,
    SocketAddrV6,
    SocketAddr,
    PathBuf,
    OsString
);

/// Wrapper for all FromStr type.
#[allow(missing_debug_implementations)]
pub struct FromStrHolder<V>(pub V);

impl<V: FromStr<Err = E> + 'static, E: std::error::Error + 'static> FromStringValue
    for FromStrHolder<V>
{
    #[inline]
    fn from_str_value(_: &mut ConfigContext<'_>, value: &str) -> Result<Self, ConfigError> {
        Ok(FromStrHolder(
            V::from_str(value).map_err(ConfigError::from_cause)?,
        ))
    }
}

macro_rules! impl_integer {
    ($($x:ident),+) => {$(
impl FromValue for $x {
    #[inline]
    fn from_value(context: &mut ConfigContext<'_>, value: ConfigValue<'_>) -> Result<Self, ConfigError> {
        use std::convert::TryFrom;
        match value {
            ConfigValue::StrRef(s) => Ok(s.parse::<$x>().map_err(ConfigError::from_cause)?),
            ConfigValue::Str(s) => Ok(s.parse::<$x>().map_err(ConfigError::from_cause)?),
            ConfigValue::Int(s) => Ok($x::try_from(s).map_err(ConfigError::from_cause)?),
            ConfigValue::Float(s) => Ok(check_f64(context, s)? as $x),
            _ => Err(context.type_mismatch::<$x>(&value)),
        }
    }
}
    )+};
}

impl_integer!(
    i8, i16, i32, i64, i128, isize, u8, u16, u32, u64, u128, usize
);

#[inline]
fn check_f64(context: &mut ConfigContext<'_>, f: f64) -> Result<f64, ConfigError> {
    if f.is_finite() {
        Ok(f)
    } else {
        Err(context.parse_error("infinite"))
    }
}
macro_rules! impl_float {
    ($($x:ident),+) => {$(
impl FromValue for $x {
    #[inline]
    #[allow(trivial_numeric_casts)]
    fn from_value(context: &mut ConfigContext<'_>, value: ConfigValue<'_>) -> Result<Self, ConfigError> {
        match value {
            ConfigValue::StrRef(s) => Ok(s.parse::<$x>().map_err(ConfigError::from_cause)?),
            ConfigValue::Str(s) => Ok(s.parse::<$x>().map_err(ConfigError::from_cause)?),
            ConfigValue::Int(s) => Ok(s as $x),
            ConfigValue::Float(s) => Ok(check_f64(context, s)? as $x),
            _ => Err(context.type_mismatch::<$x>(&value)),
        }
    }
}
    )+};
}

impl_float!(f32, f64);

#[inline]
fn parse_duration_from_str(
    context: &mut ConfigContext<'_>,
    du: &str,
) -> Result<Duration, ConfigError> {
    let mut i = 0;
    let mut multi = 1;
    let mut last = None;
    for c in du.chars().rev() {
        match c {
            'h' | 'm' | 's' if last.is_none() => {
                if c == 'm' {
                    last = Some('M');
                } else {
                    last = Some(c);
                }
            }
            'm' | 'u' | 'n' if last == Some('s') => {
                last = Some(c);
            }
            c if c.is_ascii_digit() => {
                if last.is_none() {
                    last = Some('s');
                }
                i += multi * (c as u64 - '0' as u64);
                multi *= 10;
            }
            _ => return Err(context.parse_error(du)),
        }
    }
    Ok(match last.unwrap_or('s') {
        'h' => Duration::new(i * 3600, 0),
        'M' => Duration::new(i * 60, 0),
        's' => Duration::from_secs(i),
        'm' => Duration::from_millis(i),
        'u' => Duration::from_micros(i),
        'n' => Duration::from_nanos(i),
        _ => return Err(context.parse_error(du)),
    })
}

impl FromValue for Duration {
    fn from_value(
        context: &mut ConfigContext<'_>,
        value: ConfigValue<'_>,
    ) -> Result<Self, ConfigError> {
        match value {
            ConfigValue::Str(du) => parse_duration_from_str(context, &du),
            ConfigValue::StrRef(du) => parse_duration_from_str(context, du),
            ConfigValue::Int(seconds) => Ok(Duration::from_secs(seconds as u64)),
            ConfigValue::Float(sec) => Ok(Duration::new(1, 0).mul_f64(sec)),
            _ => Err(context.type_mismatch::<Self>(&value)),
        }
    }
}

/// Implement [`FromConfig`] for enums.
///
/// ```ignore,rust
/// impl_enum!(Ordering{
///     "lt" | "less" => Ordering::Less
///     "eq" | "equal" => Ordering::Equal
///     "gt" | "greater" => Ordering::Greater
/// });
/// ```
#[macro_export]
macro_rules! impl_enum {
    ($x:path {$($($k:pat_param)|* => $v:expr)+ }) => {
        impl $crate::FromStringValue for $x {
            fn from_str_value(context: &mut $crate::ConfigContext<'_>, value: &str) -> Result<Self, $crate::ConfigError> {
                match &value.to_lowercase()[..] {
                    $($($k)|* => Ok($v),)+
                    _ => Err(context.parse_error(value)),
                }
            }
        }
    }
}

impl_enum!(Shutdown{
    "read" => Shutdown::Read
    "write" => Shutdown::Write
    "both" => Shutdown::Both
});

impl_enum!(Ordering{
    "lt" | "less" => Ordering::Less
    "eq" | "equal" => Ordering::Equal
    "gt" | "greater" => Ordering::Greater
});

#[cfg(feature = "log")]
#[doc(hidden)]
pub mod log {

    use log::*;

    impl_enum!(LevelFilter {
        "off" => LevelFilter::Off
        "trace" => LevelFilter::Trace
        "debug" => LevelFilter::Debug
        "info" => LevelFilter::Info
        "warn" => LevelFilter::Warn
        "error" => LevelFilter::Error
    });

    impl_enum!(Level {
        "trace" => Level::Trace
        "debug" => Level::Debug
        "info" => Level::Info
        "warn" => Level::Warn
        "error" => Level::Error
    });
}

#[cfg(feature = "coarsetime")]
#[doc(hidden)]
pub mod time {

    impl crate::FromValue for coarsetime::Duration {
        fn from_value(
            context: &mut crate::ConfigContext<'_>,
            value: crate::ConfigValue<'_>,
        ) -> Result<Self, crate::ConfigError> {
            std::time::Duration::from_value(context, value).map(|d| d.into())
        }
    }
}

#[cfg_attr(coverage_nightly, coverage(off))]
#[cfg(test)]
mod test {
    use crate::{Configuration, key::CacheString};

    use super::*;

    struct TestContext(Configuration, CacheString);

    impl TestContext {
        fn new() -> Self {
            Self(Configuration::new(), CacheString::new())
        }

        #[allow(single_use_lifetimes)]
        fn read<'a, T: FromConfig>(
            &mut self,
            val: impl Into<ConfigValue<'a>>,
        ) -> Result<T, ConfigError> {
            T::from_config(
                &mut self.0.source.new_context(&mut self.1),
                Some(val.into()),
            )
        }
    }

    macro_rules! should_eq {
        ($context:ident: $val:literal as $y:ty => $x:expr ) => {
            let value: Result<$y, ConfigError> = $context.read($val);
            assert_eq!($x, value.unwrap());
        };
        ($context:ident: $val:ident as $y:ty => $x:expr) => {
            let value: Result<$y, ConfigError> = $context.read($val);
            assert_eq!($x, value.unwrap());
        };
    }
    macro_rules! should_err {
        ($context:ident: $val:literal as $x:ty) => {
            let value: Result<$x, ConfigError> = $context.read($val);
            assert_eq!(true, value.is_err());
        };
    }
    macro_rules! should_valid {
        ($context:ident: $val:ident as $x:ty => $expr:expr) => {
            // println!("{}", $val);
            let value: Result<$x, ConfigError> = $context.read($val);
            assert_eq!($expr, value.is_ok());
        };
    }

    macro_rules! should_option {
        ($context:ident: $val:literal as $x:ty => $expr:expr) => {
            let v = $context.0.get::<Option<$x>>($val);
            assert_eq!($expr, v.unwrap());
        };
    }

    #[test]
    fn option_tests() {
        let context = TestContext::new();
        should_option!(context: "key" as u8 => None);
        should_option!(context: "key" as u16 => None);
        should_option!(context: "key" as u32 => None);
        should_option!(context: "key" as u64 => None);
        should_option!(context: "key" as u128 => None);
        should_option!(context: "key" as usize => None);
        should_option!(context: "key" as i8 => None);
        should_option!(context: "key" as i16 => None);
        should_option!(context: "key" as i32 => None);
        should_option!(context: "key" as i64 => None);
        should_option!(context: "key" as i128 => None);
        should_option!(context: "key" as isize => None);
        should_option!(context: "key" as String => None);
        should_option!(context: "key" as bool => None);
    }

    #[test]
    fn bool_tests() {
        let mut context = TestContext::new();

        should_eq!(context: "yes" as bool => true);
        should_eq!(context: "true" as bool => true);
        should_eq!(context: "no" as bool => false);
        should_eq!(context: "false" as bool => false);

        should_err!(context: "x" as bool);
        should_err!(context: "n" as bool);
        should_err!(context: "f" as bool);
        should_err!(context: "y" as bool);
        should_err!(context: "t" as bool);
        should_err!(context: 0u64 as bool);
        should_err!(context: 1u64 as bool);
        should_err!(context: 0.0f64 as bool);
        should_err!(context: 1.0f64 as bool);
    }

    #[quickcheck]
    fn num_tests(i: i64) {
        let mut context = TestContext::new();
        let y = format!("{}", i);
        should_eq!(context: y as i64 => i);
        should_eq!(context: i as i64 => i);
    }

    macro_rules! num_into_test {
        ($($fun:ident. $t:ty,)+) => {
            $(
            #[quickcheck]
            fn $fun(i: $t) {
                let v: ConfigValue<'static> = i.into();
                match v {
                    ConfigValue::Int(_) => {}
                    _ => assert_eq!(true, false),
                }
            }
            )+
        };
    }

    num_into_test!(
        u8_test.u8,
        u16_test.u16,
        u32_test.u32,
        i8_test.i8,
        i16_test.i16,
        i32_test.i32,
        i64_test.i64,
    );

    macro_rules! num_into_test_u {
        ($($fun:ident. $t:ty),+) => {
            $(
            #[quickcheck]
            fn $fun(i: $t) {
                let v: ConfigValue<'static> = i.into();
                match v {
                    ConfigValue::Int(_) => assert_eq!(true,  i <= i64::MAX as $t),
                    ConfigValue::Str(_) => assert_eq!(true, i > i64::MAX as $t),
                    _ => assert_eq!(true, false),
                }
            }
            )+
        };
    }

    num_into_test_u!(u64_test.u64, u128_test.u128, usize_test.usize);

    macro_rules! num_into_test_i {
        ($($fun:ident. $t:ty),+) => {
            $(
            #[quickcheck]
            fn $fun(i: $t) {
                let v: ConfigValue<'static> = i.into();
                match v {
                    ConfigValue::Int(_) => assert_eq!(true,  i <= i64::MAX as $t && i>= i64::MIN as $t),
                    ConfigValue::Str(_) => assert_eq!(true, i > i64::MAX as $t || i< i64::MIN as $t),
                    _ => assert_eq!(true, false),
                }
            }
            )+
        };
    }

    num_into_test_i!(i128_test.i128, isize_test.isize);

    #[quickcheck]
    fn i64_tests(i: i64) {
        let mut context = TestContext::new();
        should_valid!(context: i as u8 => i >= 0 && i <= (u8::MAX as i64));
        should_valid!(context: i as u16 => i >= 0 && i <= (u16::MAX as i64));
        should_valid!(context: i as u32 => i >= 0 && i <= (u32::MAX as i64));
        should_valid!(context: i as u64 => i>=0);
        should_valid!(context: i as u128 => i>=0);
        should_valid!(context: i as i8 => i >= 0 && i <= (i8::MAX as i64));
        should_valid!(context: i as i16 => i >= 0 && i <= (i16::MAX as i64));
        should_valid!(context: i as i32 => i >= 0 && i <= (i32::MAX as i64));
        should_valid!(context: i as i64 => true);
        should_valid!(context: i as i128 => true);
        should_valid!(context: i as f32 => true);
        should_valid!(context: i as f64 => true);
    }

    #[quickcheck]
    fn f64_tests(i: f64) {
        let mut context = TestContext::new();
        should_valid!(context: i as u8 => i.is_finite());
        should_valid!(context: i as u16 => i.is_finite());
        should_valid!(context: i as u32 => i.is_finite());
        should_valid!(context: i as u64 => i.is_finite());
        should_valid!(context: i as u128 => i.is_finite());
        should_valid!(context: i as i8 => i.is_finite());
        should_valid!(context: i as i16 => i.is_finite());
        should_valid!(context: i as i32 => i.is_finite());
        should_valid!(context: i as i64 => i.is_finite());
        should_valid!(context: i as i128 => i.is_finite());
        should_valid!(context: i as f32 => i.is_finite());
        should_valid!(context: i as f64 => i.is_finite());
    }

    #[test]
    fn duration_test() {
        let mut context = TestContext::new();
        should_eq!(context: "123" as Duration => Duration::new(123, 0));
        should_eq!(context: "123s" as Duration => Duration::new(123, 0));
        should_eq!(context: "10m" as Duration => Duration::new(10 * 60, 0));
        should_eq!(context: "123h" as Duration => Duration::new(123 * 3600, 0));
        should_eq!(context: "123ms" as Duration => Duration::new(0, 123 * 1_000_000));
        should_eq!(context: "123us" as Duration => Duration::new(0, 123 * 1000));
        should_eq!(context: "123ns" as Duration => Duration::new(0, 123));
        should_eq!(context: "1000ms" as Duration => Duration::new(1, 0));

        #[cfg(feature = "coarsetime")]
        {
            use coarsetime::Duration as CoarseDuration;
            should_eq!(context: "123" as CoarseDuration => CoarseDuration::new(123, 0));
            should_eq!(context: "123s" as CoarseDuration => CoarseDuration::new(123, 0));
            should_eq!(context: "10m" as CoarseDuration => CoarseDuration::new(10 * 60, 0));
            should_eq!(context: "123h" as CoarseDuration => CoarseDuration::new(123 * 3600, 0));
            should_eq!(context: "123ms" as CoarseDuration => CoarseDuration::new(0, 123 * 1_000_000));
            should_eq!(context: "123us" as CoarseDuration => CoarseDuration::new(0, 123 * 1000));
            should_eq!(context: "123ns" as CoarseDuration => CoarseDuration::new(0, 123));
            should_eq!(context: "1000ms" as CoarseDuration => CoarseDuration::new(1, 0));
        }
    }

    #[test]
    fn net_test() {
        let mut context = TestContext::new();
        should_eq!(context: "127.0.0.1" as Ipv4Addr => Ipv4Addr::new(127, 0, 0, 1));
        should_eq!(context: "::1" as Ipv6Addr => Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 1));
        should_eq!(context: "127.0.0.1:80" as  SocketAddrV4 => SocketAddrV4::new(Ipv4Addr::new(127,0,0,1), 80));
        let mut buf = PathBuf::new();
        buf.push("/var");
        should_eq!(context: "/var" as PathBuf => buf);
    }

    #[test]
    #[allow(unused_qualifications)]
    fn option_test() {
        let mut context = TestContext::new();
        let x: Result<Option<Ordering>, ConfigError> = context.read("val");
        assert!(x.is_err());
        match x.unwrap_err() {
            ConfigError::ConfigParseError(_, _) => {}
            _ => assert_eq!(true, false),
        }
    }

    #[test]
    #[allow(unused_qualifications)]
    fn into_string_test() {
        let mut context = TestContext::new();
        let v: String = context.read(1.1f64).unwrap();
        assert_eq!("1.1", v);
        let v: String = context.read(1u8).unwrap();
        assert_eq!("1", v);
        let v: String = context.read(true).unwrap();
        assert_eq!("true", v);
    }

    #[test]
    #[allow(unused_qualifications)]
    fn map_test() {
        let mut context = TestContext::new();
        let x: Result<BTreeMap<String, bool>, ConfigError> = context.read("val");
        assert!(x.is_ok());
        assert!(x.unwrap().is_empty());
        let x: Result<HashMap<String, bool>, ConfigError> = context.read("val");
        assert!(x.is_ok());
        assert!(x.unwrap().is_empty());
    }

    #[test]
    fn hash_map_with_hasher_test() {
        let mut context = TestContext::new();
        let x: Result<HashMap<String, bool>, ConfigError> = context.read("val");
        assert!(x.is_ok());
        assert!(x.unwrap().is_empty());
    }

    #[test]
    #[allow(unused_qualifications)]
    fn config_value_clone_static_works() {
        let v1 = ConfigValue::StrRef("abc");
        let v2 = v1.clone_static();
        match v2 {
            ConfigValue::Str(ref s) => assert_eq!(s, "abc"),
            _ => panic!("Expected Str variant"),
        }

        let v1 = ConfigValue::Str("def".to_string());
        let v2 = v1.clone_static();
        match v2 {
            ConfigValue::Str(ref s) => assert_eq!(s, "def"),
            _ => panic!("Expected Str variant"),
        }

        let v1 = ConfigValue::Int(42);
        let v2 = v1.clone_static();
        match v2 {
            ConfigValue::Int(i) => assert_eq!(i, 42),
            _ => panic!("Expected Int variant"),
        }

        let v1 = ConfigValue::Float(3.14);
        let v2 = v1.clone_static();
        match v2 {
            ConfigValue::Float(f) => assert!((f - 3.14).abs() < 1e-6),
            _ => panic!("Expected Float variant"),
        }

        let v1 = ConfigValue::Bool(true);
        let v2 = v1.clone_static();
        match v2 {
            ConfigValue::Bool(b) => assert!(b),
            _ => panic!("Expected Bool variant"),
        }

        #[cfg(feature = "rand")]
        {
            let v1 = ConfigValue::Rand(crate::value::RandValue::U8);
            let v2 = v1.clone_static();
            match v2 {
                ConfigValue::Rand(crate::value::RandValue::U8) => {}
                _ => panic!("Expected Rand(U8) variant"),
            }
        }
    }

    #[test]
    fn from_config_for_unit_type() {
        let mut context = TestContext::new();
        // None value should return Ok(())
        let v: Result<(), ConfigError> =
            <()>::from_config(&mut context.0.source.new_context(&mut context.1), None);
        assert!(v.is_ok());
        // Some value should also return Ok(())
        let v: Result<(), ConfigError> = <()>::from_config(
            &mut context.0.source.new_context(&mut context.1),
            Some(ConfigValue::Int(1)),
        );
        assert!(v.is_ok());
    }

    #[test]
    fn from_value_for_from_string_value() {
        struct Dummy;
        impl FromStr for Dummy {
            type Err = std::convert::Infallible;
            fn from_str(_s: &str) -> Result<Self, Self::Err> {
                Ok(Dummy)
            }
        }
        impl FromStringValue for Dummy {
            fn from_str_value(
                _context: &mut ConfigContext<'_>,
                _value: &str,
            ) -> Result<Self, ConfigError> {
                Ok(Dummy)
            }
        }

        let mut context = TestContext::new();
        // StrRef
        let v = <Dummy as FromValue>::from_value(
            &mut context.0.source.new_context(&mut context.1),
            ConfigValue::StrRef("abc"),
        );
        assert!(v.is_ok());

        // Str
        let v = <Dummy as FromValue>::from_value(
            &mut context.0.source.new_context(&mut context.1),
            ConfigValue::Str("abc".to_string()),
        );
        assert!(v.is_ok());

        // 非字符串类型应报错
        let v = <Dummy as FromValue>::from_value(
            &mut context.0.source.new_context(&mut context.1),
            ConfigValue::Int(1),
        );
        assert!(v.is_err());
    }

    #[test]
    fn from_value_for_bool() {
        let mut context = TestContext::new();

        // StrRef true/false
        let v = <bool as FromValue>::from_value(
            &mut context.0.source.new_context(&mut context.1),
            ConfigValue::StrRef("true"),
        );
        assert!(v.unwrap());
        let v = <bool as FromValue>::from_value(
            &mut context.0.source.new_context(&mut context.1),
            ConfigValue::StrRef("no"),
        );
        assert_eq!(v.unwrap(), false);

        // Str true/false
        let v = <bool as FromValue>::from_value(
            &mut context.0.source.new_context(&mut context.1),
            ConfigValue::Str("yes".to_string()),
        );
        assert_eq!(v.unwrap(), true);
        let v = <bool as FromValue>::from_value(
            &mut context.0.source.new_context(&mut context.1),
            ConfigValue::Str("false".to_string()),
        );
        assert_eq!(v.unwrap(), false);

        // Bool
        let v = <bool as FromValue>::from_value(
            &mut context.0.source.new_context(&mut context.1),
            ConfigValue::Bool(true),
        );
        assert_eq!(v.unwrap(), true);

        // 非法类型
        let v = <bool as FromValue>::from_value(
            &mut context.0.source.new_context(&mut context.1),
            ConfigValue::Int(1),
        );
        assert!(v.is_err());
    }

    #[test]
    fn from_str_holder_from_string_value() {
        type Holder = FromStrHolder<u32>;

        let mut context = TestContext::new();
        // 正常解析
        let r = Holder::from_str_value(&mut context.0.source.new_context(&mut context.1), "123");
        assert!(r.is_ok());
        assert_eq!(r.unwrap().0, 123u32);

        // 错误解析
        let r = Holder::from_str_value(&mut context.0.source.new_context(&mut context.1), "abc");
        assert!(r.is_err());
        // 错误类型为 ConfigError::ConfigCause(ParseIntError)
        match r {
            Err(ConfigError::ConfigCause(_)) => {}
            _ => panic!("Expected ConfigCause"),
        }
    }

    #[test]
    fn from_value_for_integer_types() {
        let mut context = TestContext::new();

        macro_rules! check_int {
            ($ty:ty, $val:expr, $expect:expr) => {
                let v = <$ty as FromValue>::from_value(
                    &mut context.0.source.new_context(&mut context.1),
                    ConfigValue::Int($val),
                );
                assert_eq!(v.unwrap(), $expect);

                let v = <$ty as FromValue>::from_value(
                    &mut context.0.source.new_context(&mut context.1),
                    ConfigValue::StrRef(&$val.to_string()),
                );
                assert_eq!(v.unwrap(), $expect);

                let v = <$ty as FromValue>::from_value(
                    &mut context.0.source.new_context(&mut context.1),
                    ConfigValue::Str($val.to_string()),
                );
                assert_eq!(v.unwrap(), $expect);

                let v = <$ty as FromValue>::from_value(
                    &mut context.0.source.new_context(&mut context.1),
                    ConfigValue::Float($val as f64),
                );
                assert_eq!(v.unwrap(), $expect);
            };
        }

        check_int!(i8, 7, 7i8);
        check_int!(i16, 8, 8i16);
        check_int!(i32, 9, 9i32);
        check_int!(i64, 10, 10i64);
        check_int!(u8, 11, 11u8);
        check_int!(u16, 12, 12u16);
        check_int!(u32, 13, 13u32);
        check_int!(u64, 14, 14u64);
        check_int!(usize, 15, 15usize);

        // 错误类型
        let v = <i8 as FromValue>::from_value(
            &mut context.0.source.new_context(&mut context.1),
            ConfigValue::Bool(true),
        );
        assert!(v.is_err());
    }

    #[test]
    fn from_value_for_float_types() {
        let mut context = TestContext::new();

        macro_rules! check_float {
            ($ty:ty, $val:expr, $expect:expr) => {
                let v = <$ty as FromValue>::from_value(
                    &mut context.0.source.new_context(&mut context.1),
                    ConfigValue::Float($val),
                );
                assert!((v.unwrap() - $expect).abs() < 1e-6);

                let v = <$ty as FromValue>::from_value(
                    &mut context.0.source.new_context(&mut context.1),
                    ConfigValue::StrRef(&$val.to_string()),
                );
                assert!((v.unwrap() - $expect).abs() < 1e-6);

                let v = <$ty as FromValue>::from_value(
                    &mut context.0.source.new_context(&mut context.1),
                    ConfigValue::Str($val.to_string()),
                );
                assert!((v.unwrap() - $expect).abs() < 1e-6);
            };
        }

        check_float!(f32, 1.23, 1.23f32);
        check_float!(f64, 4.56, 4.56f64);

        let v = <f32 as FromValue>::from_value(
            &mut context.0.source.new_context(&mut context.1),
            ConfigValue::Bool(true),
        );
        assert!(v.is_err());
    }

    #[test]
    fn parse_duration_from_str_cases() {
        let mut context = TestContext::new();

        // 秒
        assert_eq!(
            parse_duration_from_str(&mut context.0.source.new_context(&mut context.1), "123s")
                .unwrap(),
            Duration::new(123, 0)
        );
        // 分钟
        assert_eq!(
            parse_duration_from_str(&mut context.0.source.new_context(&mut context.1), "2m")
                .unwrap(),
            Duration::new(120, 0)
        );
        // 小时
        assert_eq!(
            parse_duration_from_str(&mut context.0.source.new_context(&mut context.1), "3h")
                .unwrap(),
            Duration::new(3 * 3600, 0)
        );
        // 毫秒
        assert_eq!(
            parse_duration_from_str(&mut context.0.source.new_context(&mut context.1), "5ms")
                .unwrap(),
            Duration::new(0, 5_000_000)
        );
        // 微秒
        assert_eq!(
            parse_duration_from_str(&mut context.0.source.new_context(&mut context.1), "7us")
                .unwrap(),
            Duration::new(0, 7_000)
        );
        // 纳秒
        assert_eq!(
            parse_duration_from_str(&mut context.0.source.new_context(&mut context.1), "9ns")
                .unwrap(),
            Duration::new(0, 9)
        );
        // 没有单位，默认为秒
        assert_eq!(
            parse_duration_from_str(&mut context.0.source.new_context(&mut context.1), "11")
                .unwrap(),
            Duration::new(11, 0)
        );
        // 错误格式
        assert!(
            parse_duration_from_str(&mut context.0.source.new_context(&mut context.1), "abc")
                .is_err()
        );
        // 不支持的单位
        assert!(
            parse_duration_from_str(&mut context.0.source.new_context(&mut context.1), "1x")
                .is_err()
        );
    }

    #[test]
    fn from_value_for_duration() {
        let mut context = TestContext::new();

        // String (seconds)
        let v = <Duration as FromValue>::from_value(
            &mut context.0.source.new_context(&mut context.1),
            ConfigValue::Str("123".to_string()),
        );
        assert_eq!(v.unwrap(), Duration::new(123, 0));

        // String (with unit)
        let v = <Duration as FromValue>::from_value(
            &mut context.0.source.new_context(&mut context.1),
            ConfigValue::Str("2m".to_string()),
        );
        assert_eq!(v.unwrap(), Duration::new(120, 0));

        // StrRef
        let v = <Duration as FromValue>::from_value(
            &mut context.0.source.new_context(&mut context.1),
            ConfigValue::StrRef("3h"),
        );
        assert_eq!(v.unwrap(), Duration::new(3 * 3600, 0));

        // Int
        let v = <Duration as FromValue>::from_value(
            &mut context.0.source.new_context(&mut context.1),
            ConfigValue::Int(7),
        );
        assert_eq!(v.unwrap(), Duration::new(7, 0));

        // Float
        let v = <Duration as FromValue>::from_value(
            &mut context.0.source.new_context(&mut context.1),
            ConfigValue::Float(1.5),
        );
        assert!((v.unwrap().as_secs_f64() - 1.5).abs() < 1e-6);

        // Invalid type
        let v = <Duration as FromValue>::from_value(
            &mut context.0.source.new_context(&mut context.1),
            ConfigValue::Bool(true),
        );
        assert!(v.is_err());
    }

    #[test]
    fn impl_enum_macro_test() {
        #[derive(Debug, PartialEq)]
        enum MyEnum {
            Foo,
            Bar,
            Baz,
        }
        impl_enum!(MyEnum {
            "foo" => MyEnum::Foo
            "bar" => MyEnum::Bar
            "baz" => MyEnum::Baz
        });

        let mut context = TestContext::new();

        // Lowercase matches
        assert_eq!(
            <MyEnum as FromStringValue>::from_str_value(
                &mut context.0.source.new_context(&mut context.1),
                "foo"
            )
            .unwrap(),
            MyEnum::Foo
        );
        assert_eq!(
            <MyEnum as FromStringValue>::from_str_value(
                &mut context.0.source.new_context(&mut context.1),
                "bar"
            )
            .unwrap(),
            MyEnum::Bar
        );
        assert_eq!(
            <MyEnum as FromStringValue>::from_str_value(
                &mut context.0.source.new_context(&mut context.1),
                "baz"
            )
            .unwrap(),
            MyEnum::Baz
        );

        // Uppercase matches (case-insensitive)
        assert_eq!(
            <MyEnum as FromStringValue>::from_str_value(
                &mut context.0.source.new_context(&mut context.1),
                "FOO"
            )
            .unwrap(),
            MyEnum::Foo
        );

        // Unknown value returns error
        let err = <MyEnum as FromStringValue>::from_str_value(
            &mut context.0.source.new_context(&mut context.1),
            "unknown",
        );
        assert!(err.is_err());
    }
}
