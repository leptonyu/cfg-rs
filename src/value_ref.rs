use crate::*;
use std::sync::*;

/// [`RefValue`] means reference of value or refreshable value,
/// it holds a value which can be updated when [`Configuration`] is refreshed.
///
/// It implements [`FromConfig`], user can use it in auto deriving config objects.
///
/// But it is not supporting **recursively** usage, following example will cause runtime error:
/// ```rust,ignore
/// #[derive(FromConfig)]
/// struct A {
///   ref_b: RefValue<B>,
/// }
/// #[derive(FromConfig)]
/// struct B {
///   ref_c: RefValue<u8>,
/// }
/// ```
#[allow(missing_debug_implementations)]
pub struct RefValue<T>(Arc<Mutex<T>>, String);

impl<T> Clone for RefValue<T> {
    fn clone(&self) -> Self {
        RefValue(self.0.clone(), self.1.clone())
    }
}

impl<T> RefValue<T> {
    fn new(k: String, v: T) -> Self {
        Self(Arc::new(Mutex::new(v)), k)
    }

    fn set(&self, v: T) -> Result<(), ConfigError> {
        *self.0.lock_c()? = v;
        Ok(())
    }

    /// Use referenced value, be careful with lock.
    pub fn with<F: FnOnce(&T) -> R, R>(&self, f: F) -> Result<R, ConfigError> {
        let g = self.0.lock_c()?;
        Ok((f)(&*g))
    }
}
impl<T: Clone> RefValue<T> {
    /// Get cloned value.
    pub fn get(&self) -> Result<T, ConfigError> {
        self.with(|v| v.clone())
    }
}

impl<T: FromConfig + Send + 'static> FromConfig for RefValue<T> {
    fn from_config(
        context: &mut ConfigContext<'_>,
        value: Option<ConfigValue<'_>>,
    ) -> Result<Self, ConfigError> {
        if context.ref_value_flag {
            return Err(ConfigError::RefValueRecursiveError);
        }
        context.ref_value_flag = true;
        let v = do_from_config(context, value);
        context.ref_value_flag = false;
        v
    }
}

#[inline]
fn do_from_config<T: FromConfig + Send + 'static>(
    context: &mut ConfigContext<'_>,
    value: Option<ConfigValue<'_>>,
) -> Result<RefValue<T>, ConfigError> {
    let v = RefValue::new(context.current_key(), T::from_config(context, value)?);
    context.as_refresher().push(v.clone())?;
    Ok(v)
}

trait Ref: Send {
    fn refresh(&self, config: &Configuration) -> Result<(), ConfigError>;
}

impl<T: FromConfig + Send> Ref for RefValue<T> {
    fn refresh(&self, config: &Configuration) -> Result<(), ConfigError> {
        self.set(config.get(&self.1)?)
    }
}

pub(crate) struct Refresher {
    max: usize,
    refs: Mutex<Vec<Box<dyn Ref + Send + 'static>>>,
}

impl Refresher {
    pub(crate) fn new() -> Self {
        Self {
            max: 1024,
            refs: Mutex::new(vec![]),
        }
    }

    fn push(&self, r: impl Ref + 'static) -> Result<(), ConfigError> {
        let mut g = self.refs.try_lock_c()?;
        if g.len() >= self.max {
          return Err(ConfigError::TooManyInstances(self.max))
        }
        g.push(Box::new(r));
        Ok(())
    }

    pub(crate) fn refresh(&self, c: &Configuration) -> Result<(), ConfigError> {
        let g = self.refs.lock_c()?;
        for i in g.iter() {
            i.refresh(c)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use std::sync::{Arc, Mutex};

    use crate::{
        source::{memory::HashSource, ConfigSource, ConfigSourceBuilder},
        *,
    };

    #[derive(FromConfig)]
    struct A {
        _ref_b: RefValue<B>,
    }
    #[derive(FromConfig)]
    struct B {
        _ref_c: RefValue<u8>,
    }

    macro_rules! should_err {
        ($v:ident) => {
            assert_eq!(true, $v.is_err());
            match $v.err().unwrap() {
                ConfigError::RefValueRecursiveError => {}
                e => {
                    println!("{:?}", e);
                    assert_eq!(true, false)
                }
            }
        };
    }

    #[test]
    fn recursive_test() {
        let config = Configuration::new();
        let v = config.get::<A>("hello");
        should_err!(v);
        let v = config.get::<RefValue<B>>("hello");
        should_err!(v);
        let v = config.get::<RefValue<RefValue<u8>>>("hello");
        should_err!(v);
    }

    struct R(Arc<Mutex<(u64, bool)>>);

    impl ConfigSource for R {
        fn name(&self) -> &str {
            "r"
        }

        fn load(&self, builder: &mut ConfigSourceBuilder<'_>) -> Result<(), ConfigError> {
            builder.set("hello", self.0.lock_c()?.0);
            Ok(())
        }

        fn allow_refresh(&self) -> bool {
            true
        }

        fn refreshable(&self) -> Result<bool, ConfigError> {
            let mut g = self.0.lock_c()?;
            let flag = g.1;
            g.1 = false;
            Ok(flag)
        }
    }

    impl R {
        fn set(&self, v: u64) {
            *self.0.lock_c().unwrap() = (v, true);
        }

        fn get(&self) -> u64 {
            self.0.lock_c().unwrap().0
        }
    }

    macro_rules! should_eq {
        ($config:ident: $r:ident. $v:ident = $i:ident) => {
            $r.set($i);
            assert_eq!(true, $config.refresh_ref().unwrap());
            assert_eq!(false, $config.refresh_ref().unwrap());
            assert_eq!($i, $v.get().unwrap());
            assert_eq!(0, $config.get::<u64>("hello").unwrap());
        };
    }

    #[test]
    fn refresh_test() {
        let r = R(Arc::new(Mutex::new((0, true))));
        assert_eq!("r", r.name());
        let config = Configuration::new()
            .register_source(R(r.0.clone()))
            .unwrap();
        let v = config.get::<RefValue<u64>>("hello").unwrap();

        for i in 0..1000 {
            should_eq!(config: r.v = i);
        }
    }

    macro_rules! should_eq_mut {
        ($config:ident: $r:ident. $v:ident = $i:ident) => {
            $r.set($i);
            assert_eq!(true, $config.refresh().unwrap());
            assert_eq!(false, $config.refresh().unwrap());
            assert_eq!($i, $v.get().unwrap());
            assert_eq!($i, $config.get::<u64>("hello").unwrap());
        };
    }
    #[test]
    fn refresh_mut_test() {
        let r = R(Arc::new(Mutex::new((0, true))));
        assert_eq!("r", r.name());
        let mut config = Configuration::new()
            .register_source(R(r.0.clone()))
            .unwrap();
        let v = config.get::<RefValue<u64>>("hello").unwrap();

        for i in 0..1000 {
            should_eq_mut!(config: r.v = i);
        }
    }

    macro_rules! should_eq_2 {
        ($config:ident: $r:ident.$s:ident. $v:ident = $i:ident) => {
            $s.set($i);
            assert_eq!(true, $config.refresh().unwrap());
            assert_eq!(false, $config.refresh().unwrap());
            assert_eq!($r.get(), $v.get().unwrap());
            $r.set($i);
            assert_eq!($i, $r.get());
            assert_ne!($r.get(), $v.get().unwrap());
            assert_eq!(true, $config.refresh().unwrap());
            assert_eq!(false, $config.refresh().unwrap());
            assert_eq!($i, $v.get().unwrap());
            assert_eq!($i, $config.get::<u64>("hello").unwrap());
        };
    }

    #[test]
    fn multiple_source_refresh_test() {
        let a = HashSource::new("name");
        let r = R(Arc::new(Mutex::new((0, true))));
        let s = R(Arc::new(Mutex::new((0, true))));
        assert_eq!("r", r.name());
        let mut config = Configuration::new()
            .register_source(a)
            .unwrap()
            .register_source(R(r.0.clone()))
            .unwrap()
            .register_source(R(s.0.clone()))
            .unwrap();
        let v = config.get::<RefValue<u64>>("hello").unwrap();

        for i in 1..1000 {
            should_eq_2!(config: r.s.v = i);
        }
    }
}
