use crate::*;
use std::sync::Arc;
use std::sync::Mutex;

/// RefValue can be updated after refresh.
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

    fn set(&self, v: T) {
        *self.0.lock().unwrap() = v;
    }

    /// Use mutable value.
    pub fn with_mut<F: FnMut(&mut T) -> R, R>(&self, mut f: F) -> R {
        let mut g = self.0.lock().unwrap();
        (f)(&mut *g)
    }
    /// Use immutable value.
    pub fn with<F: FnMut(&T) -> R, R>(&self, mut f: F) -> R {
        self.with_mut(|x| (f)(x))
    }
}

impl<T: FromConfig + 'static> FromConfig for RefValue<T> {
    fn from_config(
        context: &mut ConfigContext<'_>,
        value: Option<ConfigValue<'_>>,
    ) -> Result<Self, ConfigError> {
        let v = RefValue::new(context.current_key(), T::from_config(context, value)?);
        context.as_refresher().push(v.clone());
        Ok(v)
    }
}

trait Ref {
    fn refresh(&self, config: &Configuration) -> Result<(), ConfigError>;
}

impl<T: FromConfig> Ref for RefValue<T> {
    fn refresh(&self, config: &Configuration) -> Result<(), ConfigError> {
        Ok(self.set(config.get(&self.1)?))
    }
}

pub(crate) struct Refresher {
    refs: Mutex<Vec<Box<dyn Ref + 'static>>>,
}

impl Refresher {
    pub(crate) fn new() -> Self {
        Self {
            refs: Mutex::new(vec![]),
        }
    }

    fn push(&self, r: impl Ref + 'static) {
        let mut g = self.refs.lock().unwrap();
        g.push(Box::new(r));
    }

    pub(crate) fn refresh(&self, c: &Configuration) -> Result<(), ConfigError> {
        let g = self.refs.lock().unwrap();
        for i in g.iter() {
            i.refresh(c)?;
        }
        Ok(())
    }
}
