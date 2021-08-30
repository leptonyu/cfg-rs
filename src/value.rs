use std::{
    any::Any,
    cmp::Ordering,
    collections::{HashMap, HashSet},
    ffi::OsString,
    net::{IpAddr, Ipv4Addr, Ipv6Addr, Shutdown, SocketAddr, SocketAddrV4, SocketAddrV6},
    path::PathBuf,
    time::Duration,
};

use crate::{err::ConfigError, ConfigContext, FromConfig};

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

    #[cfg(feature = "rand")]
    pub(crate) fn normalize(v: RandValue) -> Self {
        match v {
            RandValue::U8 => ConfigValue::Int(rand::random::<u8>() as i64),
            RandValue::U16 => ConfigValue::Int(rand::random::<u16>() as i64),
            RandValue::U32 => ConfigValue::Int(rand::random::<u32>() as i64),
            RandValue::U64 => ConfigValue::Str(rand::random::<u64>().to_string()),
            RandValue::U128 => ConfigValue::Str(rand::random::<u128>().to_string()),
            RandValue::Usize => ConfigValue::Str(rand::random::<usize>().to_string()),
            RandValue::I8 => ConfigValue::Int(rand::random::<i8>() as i64),
            RandValue::I16 => ConfigValue::Int(rand::random::<i16>() as i64),
            RandValue::I32 => ConfigValue::Int(rand::random::<i32>() as i64),
            RandValue::I64 => ConfigValue::Int(rand::random::<i64>()),
            RandValue::I128 => ConfigValue::Str(rand::random::<i128>().to_string()),
            RandValue::Isize => ConfigValue::Str(rand::random::<isize>().to_string()),
        }
    }
}

impl<'a> Into<ConfigValue<'a>> for String {
    fn into(self) -> ConfigValue<'a> {
        ConfigValue::Str(self)
    }
}

impl<'a> Into<ConfigValue<'a>> for &'a str {
    fn into(self) -> ConfigValue<'a> {
        ConfigValue::StrRef(self)
    }
}

macro_rules! into_config_value_le {
    ($f:ident=$t:ident: $($x:ident),*) => {$(
        impl<'a> Into<ConfigValue<'a>> for $x {
            #[allow(trivial_numeric_casts)]
            fn into(self) -> ConfigValue<'a> {
                ConfigValue::$f(self as $t)
            }
        })*
    };
}

into_config_value_le!(Int = i64: u8, u16, u32, i8, i16, i32, i64);
into_config_value_le!(Float = f64: f32, f64);

macro_rules! into_config_value_u {
    ($($x:ident),*) => {$(
        impl<'a> Into<ConfigValue<'a>> for $x {
            fn into(self) -> ConfigValue<'a> {
                if self <= i64::MAX as $x {
                    return ConfigValue::Int(self as i64);
                }
                ConfigValue::Str(self.to_string())
            }
        })*
    };
}

into_config_value_u!(u64, u128, usize);

macro_rules! into_config_value {
    ($($x:ident),*) => {$(
        impl<'a> Into<ConfigValue<'a>> for $x {
            fn into(self) -> ConfigValue<'a> {
                if self <= i64::MAX as $x && self>= i64::MIN as $x {
                    return ConfigValue::Int(self as i64);
                }
                ConfigValue::Str(self.to_string())
            }
        })*
    };
}

into_config_value!(i128, isize);

impl<'a> Into<ConfigValue<'a>> for bool {
    fn into(self) -> ConfigValue<'a> {
        ConfigValue::Bool(self)
    }
}

#[cfg(feature = "rand")]
impl<'a> Into<ConfigValue<'a>> for RandValue {
    fn into(self) -> ConfigValue<'a> {
        ConfigValue::Rand(self)
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

impl<V: FromConfig> FromConfig for HashMap<String, V> {
    #[inline]
    fn from_config(
        context: &mut ConfigContext<'_>,
        _: Option<ConfigValue<'_>>,
    ) -> Result<Self, ConfigError> {
        let mut vs = HashMap::new();
        let list = context.collect_keys();
        for k in list.str_key {
            vs.insert(k.to_string(), context.parse_config(k, None)?);
        }
        Ok(vs)
    }
}

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
            Some(ConfigValue::StrRef(v)) if v.is_empty() => Self::empty_value(context),
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

pub trait FromStringValue: Sized + Any {
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
        Ok(<$x>::from_str(value)?)
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

macro_rules! impl_integer {
    ($($x:ident),+) => {$(
impl FromValue for $x {
    #[inline]
    fn from_value(context: &mut ConfigContext<'_>, value: ConfigValue<'_>) -> Result<Self, ConfigError> {
        use std::convert::TryFrom;
        match value {
            ConfigValue::StrRef(s) => Ok(s.parse::<$x>()?),
            ConfigValue::Str(s) => Ok(s.parse::<$x>()?),
            ConfigValue::Int(s) => Ok($x::try_from(s)?),
            ConfigValue::Float(s) => Ok(check_f64(context, s)? as $x),
            _ => Err(context.type_mismatch::<$x>(&value)),
        }
    }
}
    )+};
}

impl_integer!(i8, i16, i32, i64, i128, isize, u8, u16, u32, u64, u128, usize);

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
            ConfigValue::StrRef(s) => Ok(s.parse::<$x>()?),
            ConfigValue::Str(s) => Ok(s.parse::<$x>()?),
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
            c if ('0'..='9').contains(&c) => {
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
            ConfigValue::Float(sec) => Ok(Duration::new(0, 0).mul_f64(sec)),
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
    ($x:path {$($($k:pat)|* => $v:expr)+ }) => {
        impl $crate::value::FromStringValue for $x {
            fn from_str_value(context: &mut $crate::configuration::ConfigContext<'_>, value: &str) -> Result<Self, $crate::err::ConfigError> {
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

#[cfg(test)]
mod test {

    use crate::{key::CacheString, Configuration};

    use super::*;

    struct TestContext(Configuration, CacheString);

    impl TestContext {
        fn new() -> Self {
            Self(Configuration::new(), CacheString::new())
        }

        #[allow(single_use_lifetimes)]
        fn read<'a, T: FromValue>(
            &mut self,
            val: impl Into<ConfigValue<'a>>,
        ) -> Result<T, ConfigError> {
            T::from_value(&mut self.0.source.new_context(&mut self.1), val.into())
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
        should_eq!(context: "123ms" as Duration => Duration::new(0, 123 * 1000_000));
        should_eq!(context: "123us" as Duration => Duration::new(0, 123 * 1000));
        should_eq!(context: "123ns" as Duration => Duration::new(0, 123));
        should_eq!(context: "1000ms" as Duration => Duration::new(1, 0));
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
        let x: Result<Ordering, ConfigError> = context.read("val");
        assert_eq!(true, x.is_err());
        match x.unwrap_err() {
            ConfigError::ConfigParseError(_, _) => {}
            _ => assert_eq!(true, false),
        }
    }
}
