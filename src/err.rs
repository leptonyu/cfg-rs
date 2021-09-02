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
    /// Ref value recursive error.
    RefValueRecursiveError,
    /// Too many instances.
    TooManyInstances(usize),
    /// Lock failed.
    LockPoisoned,
    /// Config parse error with other error.
    ConfigCause(Box<dyn Error + 'static>),
}

impl<E: Error + 'static> From<E> for ConfigError {
    #[inline]
    fn from(e: E) -> Self {
        ConfigError::ConfigCause(Box::new(e))
    }
}

impl ConfigError {
    #[inline]
    pub(crate) fn try_lock_err<T>(v: TryLockError<T>) -> Option<Self> {
        match v {
            TryLockError::WouldBlock => None,
            TryLockError::Poisoned(e) => Some(Self::lock_err(e)),
        }
    }

    #[inline]
    pub(crate) fn lock_err<T>(_e: PoisonError<T>) -> Self {
        ConfigError::LockPoisoned
    }
}

pub(crate) trait ConfigLock<'a, T> {
    fn lock_c(&'a self) -> Result<MutexGuard<'a, T>, ConfigError>;

    fn try_lock_c(&'a self) -> Result<Option<MutexGuard<'a, T>>, ConfigError>;
}

impl<'a, T> ConfigLock<'a, T> for Mutex<T> {
    #[inline]
    fn lock_c(&'a self) -> Result<MutexGuard<'a, T>, ConfigError> {
        self.lock().map_err(ConfigError::lock_err)
    }

    #[inline]
    fn try_lock_c(&'a self) -> Result<Option<MutexGuard<'a, T>>, ConfigError> {
        match self.try_lock().map_err(ConfigError::try_lock_err) {
            Ok(ok) => Ok(Some(ok)),
            Err(Some(e)) => Err(e),
            _ => Ok(None),
        }
    }
}
