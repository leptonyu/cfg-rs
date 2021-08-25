//! Random source.
use crate::value::RandValue;

use super::memory::MemorySource;

/// Random source.
#[allow(missing_debug_implementations, missing_copy_implementations)]
pub struct Random;

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

    use crate::{source::memory::MemorySource, Configuration};

    use super::Random;

    #[test]
    fn env_test() {
        let config = Configuration::new().register_source(MemorySource::from(Random));
        let a = config.get::<u128>("random.u128").unwrap();
        let b = config.get::<u128>("random.u128").unwrap();
        assert_ne!(a, b);
    }
}
