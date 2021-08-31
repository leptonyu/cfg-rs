use crate::*;
use std::error::Error;
use std::path::PathBuf;

/// Configuration Error.
#[derive(Debug)]
pub enum ConfigError {
    /// Config not found.
    ConfigNotFound(String),
    /// Config not found when parsing placeholder.
    ConfigRecursiveNotFound(String),
    /// Config type mismatch.
    ConfigTypeMismatch(String, &'static str, &'static str),
    /// Config parse error.
    ConfigParseError(String, String),
    /// Config recursively parsed.
    ConfigRecursiveError(String),
    /// Config file not exists.
    ConfigFileNotExists(PathBuf),
    /// Config file not supported.
    ConfigFileNotSupported(PathBuf),
    /// Ref value cannot define recursively.
    RefValueRecursiveError,
    /// Too many instances.
    TooManyInstances(usize),
    /// Config parse error with other error.
    ConfigCause(Box<dyn Error + 'static>),
}

impl<E: Error + 'static> From<E> for ConfigError {
    fn from(e: E) -> Self {
        ConfigError::ConfigCause(Box::new(e))
    }
}

impl ConfigError {
    pub(crate) fn try_lock_err<T>(v: TryLockError<T>) -> Self {
        match v {
            TryLockError::WouldBlock => Self::RefValueRecursiveError,
            TryLockError::Poisoned(e) => Self::lock_err(e),
        }
    }

    pub(crate) fn lock_err<T>(_e: PoisonError<T>) -> Self {
        todo!()
    }
}

pub(crate) trait ConfigLock<'a, T> {
    fn lock_c(&'a self) -> Result<MutexGuard<'a, T>, ConfigError>;

    fn try_lock_c(&'a self) -> Result<MutexGuard<'a, T>, ConfigError>;
}

impl<'a, T> ConfigLock<'a, T> for Mutex<T> {
    fn lock_c(&'a self) -> Result<MutexGuard<'a, T>, ConfigError> {
        self.lock().map_err(ConfigError::lock_err)
    }

    fn try_lock_c(&'a self) -> Result<MutexGuard<'a, T>, ConfigError> {
        self.try_lock().map_err(ConfigError::try_lock_err)
    }
}
