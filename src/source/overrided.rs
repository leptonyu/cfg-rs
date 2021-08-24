use crate::*;
use std::sync::Arc;
use std::sync::Mutex;

#[derive(Clone)]
pub struct IORef<T>(Arc<Mutex<T>>, String);

impl<T>  IORef<T> {
    fn new(v: T, name: &str) -> Self {
        Self(Arc::new(Mutex::new(v)), name.to_string())
    }
}

pub(crate) struct ReloadVec(Mutex<Vec<Box<dyn Reload + 'static>>>);


impl ReloadVec {
  pub(crate) fn new() -> Self {
    Self(Mutex::new(vec![]))
  }
  pub(crate) fn push<T: FromConfig+Send+'static>(&self, r: IORef<T>) {
      let mut g = self.0.lock().unwrap();
      g.push(Box::new(r))
      
  }
}

trait Reload : Sync + Send{
    fn reload(&self, config: &Configuration) -> Result<(), ConfigError>;
}

impl<T: FromConfig + Send> Reload for IORef<T> {
    fn reload(&self, config: &Configuration) -> Result<(), ConfigError> {
        let copy = config.get(&self.1)?;
        let mut m = self.0.as_ref().lock().unwrap();
        *m = copy;
        Ok(())
    }
}
