//! File config source.
use std::{marker::PhantomData, path::PathBuf};

use crate::{ConfigError, ConfigKey, ConfigSource, ConfigValue};

use super::memory::{HashSource, PrefixHashSource};

/// File configuration source.
pub trait FileConfigSource: Send + Sync + Sized {
    /// Load source from string.
    fn load(content: String) -> Result<Self, ConfigError>;

    /// Push value
    fn push_value(self, source: &mut PrefixHashSource<'_>);

    /// Configuration file extension.
    fn ext() -> &'static str;
}

/// File source.
#[allow(missing_debug_implementations)]
pub struct FileSource<S: FileConfigSource> {
    name: String,
    path: PathBuf,
    source: HashSource,
    _data: PhantomData<S>,
}

impl<S: FileConfigSource> ConfigSource for FileSource<S> {
    fn get_value(&self, key: &ConfigKey<'_>) -> Option<ConfigValue<'_>> {
        self.source.get_value(key)
    }

    fn collect_keys<'a>(&'a self, prefix: &ConfigKey<'_>, sub: &mut crate::SubKeyList<'a>) {
        self.source.collect_keys(prefix, sub)
    }

    fn name(&self) -> &str {
        &self.name
    }
}

impl<S: FileConfigSource> FileSource<S> {
    /// Create file source.
    pub fn of(dir: &str, name: &str, profile: Option<&str>) -> Result<Self, ConfigError> {
        let mut file = PathBuf::new();
        file.push(dir);
        let path = match profile {
            Some(p) => format!("{}-{}.{}", name, p, S::ext()),
            _ => format!("{}.{}", name, S::ext()),
        };
        file.push(path);
        Self::new(file)
    }

    /// Create file source.
    pub fn new(path: PathBuf) -> Result<Self, ConfigError> {
        let source = Self::load_file(&path)?;
        let name = format!(
            "{}:{}",
            S::ext(),
            path.as_path().as_os_str().to_str().expect("Not Possible")
        );
        Ok(Self {
            name,
            path,
            source,
            _data: PhantomData,
        })
    }

    /// Reload file source.
    pub fn reload(&mut self) -> Result<(), ConfigError> {
        self.source = Self::load_file(&self.path)?;
        Ok(())
    }

    fn load_file(path: &PathBuf) -> Result<HashSource, ConfigError> {
        let mut source = HashSource::new();
        if path.exists() {
            let value = S::load(std::fs::read_to_string(path.clone())?)?;
            value.push_value(&mut source.prefixed());
        }
        Ok(source)
    }
}
