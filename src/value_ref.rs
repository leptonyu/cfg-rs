use crate::*;
use std::sync::*;

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

    fn set(&self, v: T) -> Result<(), ConfigError> {
        *self.0.lock_c()? = v;
        Ok(())
    }

    /// Use immutable value.
    pub fn with<F: FnOnce(&T) -> R, R>(&self, f: F) -> Result<R, ConfigError> {
        let g = self.0.lock_c()?;
        Ok((f)(&*g))
    }
}
impl<T: Clone> RefValue<T> {
    /// Get ref value by clone.
    pub fn get(&self) -> Result<T, ConfigError> {
        self.with(|v| v.clone())
    }
}

impl<T: FromConfig + 'static> FromConfig for RefValue<T> {
    fn from_config(
        context: &mut ConfigContext<'_>,
        value: Option<ConfigValue<'_>>,
    ) -> Result<Self, ConfigError> {
        let v = RefValue::new(context.current_key(), T::from_config(context, value)?);
        context.as_refresher().push(v.clone())?;
        Ok(v)
    }
}

trait Ref {
    fn refresh(&self, config: &Configuration) -> Result<(), ConfigError>;
}

impl<T: FromConfig> Ref for RefValue<T> {
    fn refresh(&self, config: &Configuration) -> Result<(), ConfigError> {
        self.set(config.get(&self.1)?)
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

    fn push(&self, r: impl Ref + 'static) -> Result<(), ConfigError> {
        let mut g = self.refs.try_lock_c()?;
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
