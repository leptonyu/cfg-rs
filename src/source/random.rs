//! Random source.
use super::{
    memory::{HashSourceBuilder, MemorySource},
    Loader,
};
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

impl From<Random> for MemorySource {
    fn from(_: Random) -> Self {
        MemorySource::new("random".to_string())
            .set("random.u8", RandValue::U8)
            .set("random.u16", RandValue::U16)
            .set("random.u32", RandValue::U32)
            .set("random.u64", RandValue::U64)
            .set("random.u128", RandValue::U128)
            .set("random.usize", RandValue::Usize)
            .set("random.i8", RandValue::I8)
            .set("random.i16", RandValue::I16)
            .set("random.i32", RandValue::I32)
            .set("random.i64", RandValue::I64)
            .set("random.i128", RandValue::I128)
            .set("random.isize", RandValue::Isize)
    }
}

#[cfg(test)]
mod test {

    use crate::{source::memory::MemorySource, test::TestConfigExt};

    use super::Random;

    #[test]
    fn env_test() {
        let config = MemorySource::from(Random).new_config();
        let a = config.get::<u128>("random.u128").unwrap();
        let b = config.get::<u128>("random.u128").unwrap();
        assert_ne!(a, b);
    }
}
