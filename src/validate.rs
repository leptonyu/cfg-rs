//! Validation helpers used by the `#[validate(...)]` field attributes.
///
/// These functions are called by the `FromConfig` derive to enforce ranges,
/// lengths, regex matches (with the `regex` feature), or custom checks after
/// parsing values.
use crate::ConfigError;
use std::collections::{BTreeMap, HashMap};
use std::ffi::OsString;
use std::path::PathBuf;

#[cfg(feature = "regex")]
const EMAIL_PATTERN: &str = "^[^\\s@]+@[^\\s@]+\\.[^\\s@]+$";

/// Validate a string with a regex pattern.
#[cfg(feature = "regex")]
pub fn validate_regex(field: &str, pattern: &str, value: &str) -> Result<(), ConfigError> {
    let re = regex::Regex::new(pattern).map_err(ConfigError::from_cause)?;
    if re.is_match(value) {
        Ok(())
    } else {
        Err(ConfigError::ConfigParseError(
            field.to_string(),
            format!("regex mismatch: {}", pattern),
        ))
    }
}

/// Validate a string as an email address.
#[cfg(feature = "regex")]
pub fn validate_email(field: &str, value: &str) -> Result<(), ConfigError> {
    validate_regex(field, EMAIL_PATTERN, value)
}

/// Validate range for comparable values.
pub fn validate_range<T: PartialOrd>(
    field: &str,
    value: &T,
    min: Option<&T>,
    max: Option<&T>,
) -> Result<(), ConfigError> {
    if let Some(min) = min {
        if value < min {
            return Err(ConfigError::ConfigParseError(
                field.to_string(),
                "range min".to_string(),
            ));
        }
    }
    if let Some(max) = max {
        if value > max {
            return Err(ConfigError::ConfigParseError(
                field.to_string(),
                "range max".to_string(),
            ));
        }
    }
    Ok(())
}

/// Trait for types that have a length.
pub trait ValidateLength {
    /// Returns the length of the value.
    fn validate_len(&self) -> usize;
}

impl ValidateLength for String {
    fn validate_len(&self) -> usize {
        self.len()
    }
}

impl ValidateLength for &str {
    fn validate_len(&self) -> usize {
        self.len()
    }
}

impl<T> ValidateLength for Vec<T> {
    fn validate_len(&self) -> usize {
        self.len()
    }
}

impl<K, V> ValidateLength for HashMap<K, V> {
    fn validate_len(&self) -> usize {
        self.len()
    }
}

impl<K, V> ValidateLength for BTreeMap<K, V> {
    fn validate_len(&self) -> usize {
        self.len()
    }
}

impl ValidateLength for OsString {
    fn validate_len(&self) -> usize {
        self.as_os_str().len()
    }
}

impl ValidateLength for PathBuf {
    fn validate_len(&self) -> usize {
        self.as_os_str().len()
    }
}

/// Validate length for types implementing ValidateLength.
pub fn validate_length<T: ValidateLength>(
    field: &str,
    value: &T,
    min: Option<usize>,
    max: Option<usize>,
) -> Result<(), ConfigError> {
    let len = value.validate_len();
    if let Some(min) = min {
        if len < min {
            return Err(ConfigError::ConfigParseError(
                field.to_string(),
                "length min".to_string(),
            ));
        }
    }
    if let Some(max) = max {
        if len > max {
            return Err(ConfigError::ConfigParseError(
                field.to_string(),
                "length max".to_string(),
            ));
        }
    }
    Ok(())
}

/// Validate that a value implementing ValidateLength is not empty.
pub fn validate_not_empty<T: ValidateLength>(field: &str, value: &T) -> Result<(), ConfigError> {
    if value.validate_len() == 0 {
        return Err(ConfigError::ConfigParseError(
            field.to_string(),
            "length empty".to_string(),
        ));
    }
    Ok(())
}

/// Validate with a custom function.
pub fn validate_custom<T, F>(field: &str, value: &T, f: F) -> Result<(), ConfigError>
where
    F: Fn(&T) -> Result<(), ConfigError>,
{
    let _ = field;
    f(value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn validate_range_numeric_and_duration() {
        assert!(validate_range("u8", &5u8, Some(&1u8), Some(&10u8)).is_ok());
        assert!(validate_range("u8", &0u8, Some(&1u8), None).is_err());
        assert!(validate_range("u16", &5u16, Some(&1u16), Some(&10u16)).is_ok());
        assert!(validate_range("u32", &5u32, Some(&1u32), Some(&10u32)).is_ok());
        assert!(validate_range("u64", &5u64, Some(&1u64), Some(&10u64)).is_ok());
        assert!(validate_range("u128", &5u128, Some(&1u128), Some(&10u128)).is_ok());
        assert!(validate_range("usize", &5usize, Some(&1usize), Some(&10usize)).is_ok());

        assert!(validate_range("i8", &-2i8, Some(&-5i8), Some(&-1i8)).is_ok());
        assert!(validate_range("i16", &-2i16, Some(&-5i16), Some(&-1i16)).is_ok());
        assert!(validate_range("i32", &-2i32, Some(&-5i32), Some(&-1i32)).is_ok());
        assert!(validate_range("i64", &-2i64, Some(&-5i64), Some(&-1i64)).is_ok());
        assert!(validate_range("i128", &-2i128, Some(&-5i128), Some(&-1i128)).is_ok());
        assert!(validate_range("isize", &-2isize, Some(&-5isize), Some(&-1isize)).is_ok());

        assert!(validate_range("f32", &1.5f32, Some(&0.5f32), Some(&2.0f32)).is_ok());
        assert!(validate_range("f64", &1.5f64, Some(&0.5f64), Some(&2.0f64)).is_ok());

        let dur = Duration::from_secs(5);
        assert!(validate_range("dur", &dur, Some(&Duration::from_secs(1)), None).is_ok());
        assert!(validate_range("dur", &dur, Some(&Duration::from_secs(6)), None).is_err());
    }

    #[test]
    fn validate_length_supported_types() {
        let s = "hello".to_string();
        assert!(validate_length("s", &s, Some(1), Some(10)).is_ok());
        assert!(validate_length("s", &s, Some(6), None).is_err());

        let v = vec![1, 2, 3];
        assert!(validate_length("v", &v, Some(1), Some(3)).is_ok());
        assert!(validate_length("v", &v, None, Some(2)).is_err());

        let mut hm = HashMap::new();
        hm.insert("a", 1);
        assert!(validate_length("hm", &hm, Some(1), None).is_ok());

        let mut bm = BTreeMap::new();
        bm.insert("a", 1);
        bm.insert("b", 2);
        assert!(validate_length("bm", &bm, Some(1), Some(2)).is_ok());

        let os = OsString::from("abc");
        assert!(validate_length("os", &os, Some(1), Some(3)).is_ok());

        let pb = PathBuf::from("/tmp");
        assert!(validate_length("pb", &pb, Some(1), None).is_ok());
    }

    #[test]
    fn validate_not_empty_supported_types() {
        let s = "hello".to_string();
        assert!(validate_not_empty("s", &s).is_ok());
        let s_empty = "".to_string();
        assert!(validate_not_empty("s", &s_empty).is_err());

        let v = vec![1, 2, 3];
        assert!(validate_not_empty("v", &v).is_ok());
        let v_empty: Vec<u8> = Vec::new();
        assert!(validate_not_empty("v", &v_empty).is_err());

        let mut hm = HashMap::new();
        hm.insert("a", 1);
        assert!(validate_not_empty("hm", &hm).is_ok());
        let hm_empty: HashMap<&str, u8> = HashMap::new();
        assert!(validate_not_empty("hm", &hm_empty).is_err());

        let os = OsString::from("abc");
        assert!(validate_not_empty("os", &os).is_ok());
        let os_empty = OsString::from("");
        assert!(validate_not_empty("os", &os_empty).is_err());

        let pb = PathBuf::from("/tmp");
        assert!(validate_not_empty("pb", &pb).is_ok());
        let pb_empty = PathBuf::from("");
        assert!(validate_not_empty("pb", &pb_empty).is_err());
    }
}
