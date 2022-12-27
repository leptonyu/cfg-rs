//! Random source.
use std::cell::RefCell;

use rand_chacha::{
    rand_core::{OsRng, RngCore, SeedableRng},
    ChaCha12Rng,
};

use super::{memory::ConfigSourceBuilder, ConfigSource};
use crate::{value::RandValue, ConfigError, ConfigValue};

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

thread_local! {
    static RND: RefCell<ChaCha12Rng> = RefCell::new( ChaCha12Rng::from_rng(OsRng).unwrap());
}

#[inline]
fn get_rand<R, F: Fn(&mut ChaCha12Rng) -> R>(f: F) -> R {
    RND.with(move |x| (f)(&mut x.borrow_mut()))
}

macro_rules! get_val {
    ($($f:ident.$n:literal),+) => {$(
        #[inline]
        fn $f<R, F: Fn(&[u8; $n]) -> R>(f: F) -> R {
            get_rand(|c| {
                let mut x = [0; $n];
                c.fill_bytes(&mut x);
                (f)(&x)
            })
        }
        )+};
}

get_val!(get_1.1, get_2.2, get_4.4, get_8.8, get_16.16);

impl RandValue {
    pub(crate) fn normalize(self) -> ConfigValue<'static> {
        match self {
            RandValue::U8 => get_rand(|f| f.next_u32() as u8).into(),
            RandValue::U16 => get_rand(|f| f.next_u32() as u16).into(),
            RandValue::U32 => get_rand(|f| f.next_u32()).into(),
            RandValue::U64 => get_rand(|f| f.next_u64()).into(),
            RandValue::U128 => get_16(|f| u128::from_le_bytes(*f)).into(),
            RandValue::Usize => get_8(|f| usize::from_le_bytes(*f)).into(),
            RandValue::I8 => get_1(|f| i8::from_le_bytes(*f)).into(),
            RandValue::I16 => get_2(|f| i16::from_le_bytes(*f)).into(),
            RandValue::I32 => get_4(|f| i32::from_le_bytes(*f)).into(),
            RandValue::I64 => get_8(|f| i64::from_le_bytes(*f)).into(),
            RandValue::I128 => get_16(|f| i128::from_le_bytes(*f)).into(),
            RandValue::Isize => get_8(|f| isize::from_le_bytes(*f)).into(),
        }
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
        assert!(config.get::<u8>("random.u8").is_ok());
        assert!(config.get::<u16>("random.u16").is_ok());
        assert!(config.get::<u32>("random.u32").is_ok());
        assert!(config.get::<u64>("random.u64").is_ok());
        assert!(config.get::<u128>("random.u128").is_ok());
        assert!(config.get::<usize>("random.usize").is_ok());
        assert!(config.get::<i8>("random.i8").is_ok());
        assert!(config.get::<i16>("random.i16").is_ok());
        assert!(config.get::<i32>("random.i32").is_ok());
        assert!(config.get::<i64>("random.i64").is_ok());
        assert!(config.get::<i128>("random.i128").is_ok());
        assert!(config.get::<isize>("random.isize").is_ok());
    }
}
