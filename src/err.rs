use crate::*;
use std::error::Error;
use std::fmt::Display;
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

impl ConfigError {
    #[inline]
    /// Creates a `ConfigError` from another error type.
    pub fn from_cause<E: Error + 'static>(e: E) -> Self {
        ConfigError::ConfigCause(Box::new(e))
    }
}

impl Error for ConfigError {}

impl Display for ConfigError {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigError::ConfigNotFound(key) => {
                write!(f, "Configuration not found: {}", key)
            }
            ConfigError::ConfigRecursiveNotFound(key) => {
                write!(f, "Configuration recursive not found: {}", key)
            }
            ConfigError::ConfigTypeMismatch(key, expected, found) => {
                write!(
                    f,
                    "Configuration type mismatch for key '{}': expected {}, found {}",
                    key, expected, found
                )
            }
            ConfigError::ConfigParseError(key, msg) => {
                write!(f, "Configuration parse error for key '{}': {}", key, msg)
            }
            ConfigError::ConfigRecursiveError(key) => {
                write!(f, "Configuration recursive error for key '{}'", key)
            }
            ConfigError::ConfigFileNotExists(path) => {
                write!(f, "Configuration file does not exist: {:?}", path)
            }
            ConfigError::ConfigFileNotSupported(path) => {
                write!(f, "Configuration file not supported: {:?}", path)
            }
            ConfigError::RefValueRecursiveError => {
                write!(f, "Reference value recursive error")
            }
            ConfigError::TooManyInstances(count) => {
                write!(f, "Too many instances: {}", count)
            }
            ConfigError::LockPoisoned => {
                write!(f, "Lock poisoned")
            }
            ConfigError::ConfigCause(e) => {
                write!(f, "Configuration error caused by: {}", e)
            }
        }
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
        let v = self.try_lock().map_err(ConfigError::try_lock_err);
        match v {
            Ok(ok) => Ok(Some(ok)),
            Err(Some(e)) => Err(e),
            _ => Ok(None),
        }
    }
}

#[cfg_attr(coverage_nightly, coverage(off))]
#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};
    use std::thread;
    use std::time::Duration;

    #[test]
    fn display_config_not_found() {
        let e = ConfigError::ConfigNotFound("db.url".into());
        assert_eq!(format!("{}", e), "Configuration not found: db.url");
    }

    #[test]
    fn display_config_recursive_not_found() {
        let e = ConfigError::ConfigRecursiveNotFound("${missing}".into());
        assert_eq!(
            format!("{}", e),
            "Configuration recursive not found: ${missing}"
        );
    }

    #[test]
    fn display_config_type_mismatch() {
        let e = ConfigError::ConfigTypeMismatch("app.port".into(), "u16", "String");
        assert_eq!(
            format!("{}", e),
            "Configuration type mismatch for key 'app.port': expected u16, found String"
        );
    }

    #[test]
    fn display_config_parse_error() {
        let e = ConfigError::ConfigParseError("app.timeout".into(), "invalid number".into());
        assert_eq!(
            format!("{}", e),
            "Configuration parse error for key 'app.timeout': invalid number"
        );
    }

    #[test]
    fn display_config_recursive_error() {
        let e = ConfigError::ConfigRecursiveError("a -> b -> a".into());
        assert_eq!(
            format!("{}", e),
            "Configuration recursive error for key 'a -> b -> a'"
        );
    }

    #[test]
    fn display_config_file_not_exists_contains_path() {
        let p = PathBuf::from("/tmp/file.toml");
        let s = format!("{}", ConfigError::ConfigFileNotExists(p.clone()));
        assert!(s.starts_with("Configuration file does not exist: "));
        // Be tolerant to potential platform/debug formatting differences by checking containment.
        assert!(s.contains("/tmp/file.toml"), "{}", s);
    }

    #[test]
    fn display_config_file_not_supported_contains_path() {
        let p = PathBuf::from("/tmp/file.unsupported");
        let s = format!("{}", ConfigError::ConfigFileNotSupported(p.clone()));
        assert!(s.starts_with("Configuration file not supported: "));
        assert!(s.contains("/tmp/file.unsupported"), "{}", s);
    }

    #[test]
    fn display_ref_value_recursive_error() {
        let e = ConfigError::RefValueRecursiveError;
        assert_eq!(format!("{}", e), "Reference value recursive error");
    }

    #[test]
    fn display_too_many_instances() {
        let e = ConfigError::TooManyInstances(42);
        assert_eq!(format!("{}", e), "Too many instances: 42");
    }

    #[test]
    fn display_lock_poisoned() {
        let e = ConfigError::LockPoisoned;
        assert_eq!(format!("{}", e), "Lock poisoned");
    }

    #[test]
    fn display_config_cause() {
        let io_err = std::io::Error::other("io");
        let e = ConfigError::from_cause(io_err);
        assert_eq!(format!("{}", e), "Configuration error caused by: io");
    }

    #[test]
    fn config_error_from_converts_to_configcause() {
        let io_err = std::io::Error::other("io");
        let ce = ConfigError::from_cause(io_err);
        match ce {
            ConfigError::ConfigCause(_) => {}
            _ => panic!("Expected ConfigCause variant"),
        }
    }

    #[test]
    fn try_lock_err_variants_and_poison_detection() {
        // WouldBlock -> None
        assert!(ConfigError::try_lock_err(TryLockError::WouldBlock::<()>).is_none());

        // Create a poisoned mutex by panicking while holding the lock in another thread.
        let m = Arc::new(Mutex::new(()));
        let mm = m.clone();
        let h = thread::spawn(move || {
            let _g = mm.lock().unwrap();
            panic!("poison");
        });
        // join to ensure the panic happened and the mutex is poisoned
        let _ = h.join();

        // 用作用域包裹，确保 borrow 生命周期不会超出 m 的作用域
        {
            let try_result = m.try_lock();
            match try_result {
                Err(e) => {
                    // Ensure ConfigError::try_lock_err maps Poisoned -> Some(LockPoisoned)
                    let opt = ConfigError::try_lock_err(e);
                    assert!(opt.is_some());
                    if let Some(err) = opt {
                        match err {
                            ConfigError::LockPoisoned => {}
                            _ => panic!("Expected LockPoisoned"),
                        }
                    }
                }
                Ok(_) => panic!("Expected poisoned mutex"),
            }
        }
    }

    #[test]
    fn configlock_mutex_lock_c_and_try_lock_c_behaviour() {
        // lock_c on fresh mutex should succeed
        let m_ok = Mutex::new(1);
        assert!(m_ok.lock_c().is_ok());

        // try_lock_c returns None when another thread holds the lock (WouldBlock)
        let m_block = Arc::new(Mutex::new(0));
        let m_block_c = m_block.clone();
        let handle = thread::spawn(move || {
            let _g = m_block_c.lock().unwrap();
            thread::sleep(Duration::from_millis(200));
            // guard drops here
        });
        // give spawned thread time to acquire the lock
        thread::sleep(Duration::from_millis(10));
        match m_block.try_lock_c().unwrap() {
            None => {} // expected
            Some(_) => panic!("Expected None when mutex is held by another thread"),
        }
        handle.join().unwrap();

        // Now create a poisoned mutex and ensure lock_c / try_lock_c return LockPoisoned
        let m_poison = Arc::new(Mutex::new(()));
        let mm = m_poison.clone();
        let h2 = thread::spawn(move || {
            let _g = mm.lock().unwrap();
            panic!("poison");
        });
        let _ = h2.join();

        // try_lock_c should return Err(ConfigError::LockPoisoned)
        assert!(matches!(
            m_poison.try_lock_c(),
            Err(ConfigError::LockPoisoned)
        ));

        // lock_c should return Err(ConfigError::LockPoisoned)
        assert!(matches!(m_poison.lock_c(), Err(ConfigError::LockPoisoned)));
    }
}
