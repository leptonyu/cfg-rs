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
    /// Config parse error with other error.
    ConfigCause(Box<dyn Error + 'static>),
}

impl<E: Error + 'static> From<E> for ConfigError {
    fn from(e: E) -> Self {
        ConfigError::ConfigCause(Box::new(e))
    }
}
