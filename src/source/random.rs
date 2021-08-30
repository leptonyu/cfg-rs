//! Random source.
use super::{memory::ConfigSourceBuilder, ConfigSource};
use crate::{value::RandValue, ConfigError};

/// Random source.
#[allow(missing_debug_implementations, missing_copy_implementations)]
pub(crate) struct Random;

impl ConfigSource for Random {
    fn name(&self) -> &str {
        "random_generator"
    }

    fn load(&self, source: &mut ConfigSourceBuilder<'_>) -> Result<(), ConfigError> {
        source.set("random.u8", RandValue::U8);
        source.set("random.u16", RandValue::U16);
        source.set("random.u32", RandValue::U32);
        source.set("random.u64", RandValue::U64);
        source.set("random.u128", RandValue::U128);
        source.set("random.usize", RandValue::Usize);
        source.set("random.i8", RandValue::I8);
        source.set("random.i16", RandValue::I16);
        source.set("random.i32", RandValue::I32);
        source.set("random.i64", RandValue::I64);
        source.set("random.i128", RandValue::I128);
        source.set("random.isize", RandValue::Isize);
        Ok(())
    }
}

#[cfg(test)]
mod test {

    use crate::test::TestConfigExt;

    use super::Random;

    #[test]
    fn env_test() {
        let config = Random.new_config();
        let a = config.get::<u128>("random.u128").unwrap();
        let b = config.get::<u128>("random.u128").unwrap();
        assert_ne!(a, b);
    }

    #[test]
    fn value_test() {
        let config = Random.new_config();
        assert_eq!(true, config.get::<u8>("random.u8").is_ok());
        assert_eq!(true, config.get::<u16>("random.u16").is_ok());
        assert_eq!(true, config.get::<u32>("random.u32").is_ok());
        assert_eq!(true, config.get::<u64>("random.u64").is_ok());
        assert_eq!(true, config.get::<u128>("random.u128").is_ok());
        assert_eq!(true, config.get::<usize>("random.usize").is_ok());
        assert_eq!(true, config.get::<i8>("random.i8").is_ok());
        assert_eq!(true, config.get::<i16>("random.i16").is_ok());
        assert_eq!(true, config.get::<i32>("random.i32").is_ok());
        assert_eq!(true, config.get::<i64>("random.i64").is_ok());
        assert_eq!(true, config.get::<i128>("random.i128").is_ok());
        assert_eq!(true, config.get::<isize>("random.isize").is_ok());
    }
}
