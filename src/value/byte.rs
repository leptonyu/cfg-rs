use crate::{ConfigContext, FromValue};
use std::convert::TryFrom;

/// Byte size value in bytes. Use `u64` for platform independence.
#[derive(Debug, Clone, Copy)]
pub struct ByteSize(u64);

impl std::ops::Deref for ByteSize {
    type Target = u64;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<ByteSize> for u64 {
    fn from(v: ByteSize) -> Self {
        v.0
    }
}

impl From<ByteSize> for usize {
    fn from(v: ByteSize) -> Self {
        usize::try_from(v.0).expect("ByteSize value doesn't fit usize")
    }
}

#[inline]
fn parse_byte_size_from_str(
    context: &mut ConfigContext<'_>,
    value: &str,
) -> Result<ByteSize, crate::err::ConfigError> {
    let v = value.trim();
    if v.is_empty() {
        return Err(context.parse_error(value));
    }

    let mut split = 0usize;
    for (idx, c) in v.char_indices() {
        if c.is_ascii_digit() {
            split = idx + c.len_utf8();
        } else {
            break;
        }
    }
    if split == 0 {
        return Err(context.parse_error(value));
    }

    let num = v[..split]
        .parse::<u64>()
        .map_err(crate::err::ConfigError::from_cause)?;
    let unit = v[split..].trim().to_ascii_lowercase();
    let mul = match unit.as_str() {
        "" | "b" => 1,
        "kb" => 1024,
        "mb" => 1024u64.pow(2),
        "gb" => 1024u64.pow(3),
        "tb" => 1024u64.pow(4),
        "pb" => 1024u64.pow(5),
        _ => return Err(context.parse_error(value)),
    };
    num.checked_mul(mul)
        .map(ByteSize)
        .ok_or_else(|| context.parse_error(value))
}

impl FromValue for ByteSize {
    fn from_value(
        context: &mut ConfigContext<'_>,
        value: crate::ConfigValue<'_>,
    ) -> Result<Self, crate::err::ConfigError> {
        match value {
            crate::ConfigValue::Str(v) => parse_byte_size_from_str(context, &v),
            crate::ConfigValue::StrRef(v) => parse_byte_size_from_str(context, v),
            crate::ConfigValue::Int(v) => Ok(ByteSize(
                u64::try_from(v).map_err(crate::err::ConfigError::from_cause)?,
            )),
            _ => Err(context.type_mismatch::<Self>(&value)),
        }
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod test {
    use super::*;
    use crate::ConfigValue;
    use crate::{Configuration, key::CacheString};

    struct TestContext(Configuration, CacheString);

    impl TestContext {
        fn new() -> Self {
            Self(Configuration::new(), CacheString::new())
        }
    }

    #[test]
    fn parse_byte_size_from_str_cases() {
        let mut context = TestContext::new();

        assert_eq!(
            *parse_byte_size_from_str(&mut context.0.source.new_context(&mut context.1), "1")
                .unwrap(),
            1
        );
        assert_eq!(
            *parse_byte_size_from_str(&mut context.0.source.new_context(&mut context.1), "1b")
                .unwrap(),
            1
        );
        assert_eq!(
            *parse_byte_size_from_str(&mut context.0.source.new_context(&mut context.1), "2KB")
                .unwrap(),
            2 * 1024
        );
        assert_eq!(
            *parse_byte_size_from_str(&mut context.0.source.new_context(&mut context.1), "3mb")
                .unwrap(),
            3 * 1024 * 1024
        );
        assert_eq!(
            *parse_byte_size_from_str(&mut context.0.source.new_context(&mut context.1), "4Gb")
                .unwrap(),
            4 * 1024 * 1024 * 1024
        );
        assert_eq!(
            *parse_byte_size_from_str(&mut context.0.source.new_context(&mut context.1), "1tb")
                .unwrap(),
            1024u64.pow(4)
        );
        assert_eq!(
            *parse_byte_size_from_str(&mut context.0.source.new_context(&mut context.1), "1PB")
                .unwrap(),
            1024u64.pow(5)
        );

        assert!(
            parse_byte_size_from_str(&mut context.0.source.new_context(&mut context.1), "abc")
                .is_err()
        );
        assert!(
            parse_byte_size_from_str(&mut context.0.source.new_context(&mut context.1), "1.5kb")
                .is_err()
        );
        assert!(
            parse_byte_size_from_str(&mut context.0.source.new_context(&mut context.1), "1zb")
                .is_err()
        );
        assert!(
            parse_byte_size_from_str(&mut context.0.source.new_context(&mut context.1), "   ")
                .is_err()
        );
        assert!(
            parse_byte_size_from_str(
                &mut context.0.source.new_context(&mut context.1),
                "18446744073709551616b"
            )
            .is_err()
        );
        assert!(
            parse_byte_size_from_str(
                &mut context.0.source.new_context(&mut context.1),
                &format!("{}pb", u64::MAX)
            )
            .is_err()
        );
    }

    #[test]
    fn from_value_for_byte_size() {
        let mut context = TestContext::new();

        let v = <ByteSize as FromValue>::from_value(
            &mut context.0.source.new_context(&mut context.1),
            ConfigValue::Str("1kb".to_string()),
        );
        assert_eq!(*v.unwrap(), 1024);

        let v = <ByteSize as FromValue>::from_value(
            &mut context.0.source.new_context(&mut context.1),
            ConfigValue::StrRef("2MB"),
        );
        assert_eq!(*v.unwrap(), 2 * 1024 * 1024);

        let v = <ByteSize as FromValue>::from_value(
            &mut context.0.source.new_context(&mut context.1),
            ConfigValue::Int(7),
        );
        assert_eq!(*v.unwrap(), 7);

        let v = <ByteSize as FromValue>::from_value(
            &mut context.0.source.new_context(&mut context.1),
            ConfigValue::Int(-1),
        );
        assert!(v.is_err());

        let v = <ByteSize as FromValue>::from_value(
            &mut context.0.source.new_context(&mut context.1),
            ConfigValue::Float(1.0),
        );
        assert!(v.is_err());
    }

    #[test]
    fn byte_size_accessors() {
        let v = ByteSize(1024);
        assert_eq!(*v, 1024);
        let n: usize = v.into();
        assert_eq!(n, 1024);
        let n64: u64 = v.into();
        assert_eq!(n64, 1024);
    }
}
