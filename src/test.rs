use crate::source::ConfigSource;
use crate::source::memory::HashSource;
use crate::*;

pub(crate) trait TestConfigExt: ConfigSource + Sized + 'static {
    fn new_config(self) -> Configuration {
        Configuration::new().register_source(self).unwrap()
    }
}

impl<C: ConfigSource + 'static> TestConfigExt for C {}

type R<V> = Result<V, ConfigError>;
use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;

#[derive(Debug, FromConfig)]
#[config(crate = "crate")]
struct ConfigSuit {
    #[config(name = "val")]
    int: IntSuit,
    arr: Vec<String>,
    brr: Vec<Vec<String>>,
    #[config(name = "val")]
    map: HashMap<String, usize>,
    #[config(name = "val")]
    bmap: BTreeMap<String, usize>,
    #[config(name = "map")]
    bap: HashMap<String, Vec<bool>>,
    crr: Vec<FloatSuit>,
    err: R<u8>,
}
#[derive(Debug, FromConfig)]
#[config(crate = "crate")]
struct FloatSuit {
    v1: f32,
    v2: f64,
}

#[derive(Debug, FromConfig)]
#[config(crate = "crate")]
struct IntSuit {
    v1: u8,
    v2: u16,
    v3: u32,
}

#[allow(dead_code)]
pub(crate) fn source_test_suit(src: impl ConfigSource + 'static) -> Result<(), ConfigError> {
    let config = src.new_config();
    let v: ConfigSuit = config.get("suit")?;
    assert_eq!(vec!["a0", "a1", "a2"], v.arr);
    assert_eq!(Some(&vec![true]), v.bap.get("b1"));
    assert_eq!(Some(&vec![true, false]), v.bap.get("b2"));
    let brr = vec!["b00"];
    assert_eq!(vec![brr], v.brr);
    for i in 1..=3 {
        assert_eq!(Some(&i), v.map.get(&format!("v{}", i)));
        assert_eq!(Some(&i), v.bmap.get(&format!("v{}", i)));
    }
    assert_eq!(1, v.int.v1);
    assert_eq!(2, v.int.v2);
    assert_eq!(3, v.int.v3);

    assert_eq!(1, v.crr.len());
    let crr = &v.crr[0];
    assert_eq!(1.0, crr.v1);
    assert_eq!(2.0, crr.v2);
    assert!(v.err.is_err());
    Ok(())
}

#[test]
fn in_memory_test() {
    source_test_suit(
        HashSource::new("test")
            .set("suit.val.v1", "1")
            .set("suit.val.v2", "2")
            .set("suit.val.v3", "3")
            .set("suit.arr[0]", "a0")
            .set("suit.arr[1]", "a1")
            .set("suit.arr[2]", "a2")
            .set("suit.map.b1[0]", "true")
            .set("suit.map.b2[0]", "true")
            .set("suit.map.b2[1]", "false")
            .set("suit.crr[0].v1", "1.0")
            .set("suit.crr[0].v2", "2.0")
            .set("suit.brr[0][0]", "b00"),
    )
    .unwrap();
}

#[allow(dead_code)]
#[derive(Debug, FromConfig)]
#[config(crate = "crate", prefix = "validate")]
struct ValidateCfg {
    #[validate(range(min = 1, max = 3), message = "port must be between 1 and 3")]
    port: u8,
    #[validate(length(min = 1, max = 5))]
    name: String,
    #[validate(not_empty)]
    alias: String,
    #[validate(length(min = 1, max = 2))]
    tags: Vec<String>,
    #[validate(length(min = 1, max = 10))]
    path: PathBuf,
    #[validate(custom = check_threads)]
    threads: usize,
    #[validate(length(min = 1, max = 3))]
    optional: Option<String>,
    #[cfg(feature = "regex")]
    #[validate(regex = "^u[a-z]+$")]
    user: String,
    #[cfg(feature = "regex")]
    #[validate(regex = "^[^@\\s]+@[^@\\s]+\\.[^@\\s]+$")]
    email: String,
}

fn check_threads(v: &usize) -> Result<(), String> {
    if *v == 0 {
        return Err("threads must be > 0".to_string());
    }
    Ok(())
}

#[test]
fn validate_annotations_happy_path() {
    let config = HashSource::new("validate")
        .set("validate.port", "2")
        .set("validate.name", "rust")
        .set("validate.alias", "rs")
        .set("validate.tags[0]", "a")
        .set("validate.path", "/tmp")
        .set("validate.threads", "2")
        .set("validate.user", "user")
        .set("validate.email", "user@example.com")
        .set("validate.optional", "opt")
        .new_config();

    let cfg: ValidateCfg = config.get_predefined().unwrap();
    assert_eq!(cfg.port, 2);
    assert_eq!(cfg.name, "rust");
    assert_eq!(cfg.tags.len(), 1);
    assert_eq!(cfg.threads, 2);
}

#[test]
fn validate_annotations_custom_error() {
    let config = HashSource::new("validate")
        .set("validate.port", "2")
        .set("validate.name", "rust")
        .set("validate.alias", "rs")
        .set("validate.tags[0]", "a")
        .set("validate.path", "/tmp")
        .set("validate.threads", "0")
        .set("validate.user", "user")
        .set("validate.email", "user@example.com")
        .new_config();

    let err = config.get_predefined::<ValidateCfg>().unwrap_err();
    match err {
        ConfigError::ConfigParseError(key, _) => assert_eq!(key, "validate.threads"),
        _ => panic!("unexpected error: {:?}", err),
    }
}

#[cfg(feature = "regex")]
#[test]
fn validate_annotations_regex_email_error() {
    let config = HashSource::new("validate")
        .set("validate.port", "2")
        .set("validate.name", "rust")
        .set("validate.alias", "rs")
        .set("validate.tags[0]", "a")
        .set("validate.path", "/tmp")
        .set("validate.threads", "2")
        .set("validate.user", "BAD")
        .set("validate.email", "not-an-email")
        .new_config();

    let err = config.get_predefined::<ValidateCfg>().unwrap_err();
    match err {
        ConfigError::ConfigParseError(key, _) => assert_eq!(key, "validate.user"),
        _ => panic!("unexpected error: {:?}", err),
    }
}

#[test]
fn validate_annotations_not_empty_error() {
    let config = HashSource::new("validate")
        .set("validate.port", "2")
        .set("validate.name", "rust")
        .set("validate.alias", "")
        .set("validate.tags[0]", "a")
        .set("validate.path", "/tmp")
        .set("validate.threads", "2")
        .set("validate.user", "user")
        .set("validate.email", "user@example.com")
        .new_config();

    let err = config.get_predefined::<ValidateCfg>().unwrap_err();
    match err {
        ConfigError::ConfigParseError(key, _) => assert_eq!(key, "validate.alias"),
        _ => panic!("unexpected error: {:?}", err),
    }
}

#[derive(Debug, FromConfig)]
#[config(crate = "crate")]
#[allow(dead_code)]
struct MultiRuleValidation {
    #[validate(length(min = 0, max = 4), not_empty, message = "empty")]
    name: String,
}

#[test]
fn test_validate_multiple_rules_in_one_attribute() {
    let mut map = HashMap::new();
    map.insert("name", "");

    let err =
        from_map::<MultiRuleValidation, _, _, _>(map, "").expect_err("expected validation failure");
    match err {
        ConfigError::ConfigParseError(field, message) => {
            assert_eq!(field, "name");
            assert_eq!(message, "empty");
        }
        other => panic!("unexpected error: {:?}", other),
    }
}
