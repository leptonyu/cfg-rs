//! Random source.
use super::{memory::HashSourceBuilder, Loader};
use crate::{value::RandValue, ConfigError};

/// Random source.
#[allow(missing_debug_implementations, missing_copy_implementations)]
pub struct Random;

impl Loader for Random {
    fn load(&self, source: &mut HashSourceBuilder<'_>) -> Result<(), ConfigError> {
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
}
