//! File config source.
use std::{marker::PhantomData, path::PathBuf};

use crate::{ConfigError, ConfigKey, ConfigSource, ConfigValue, PartialKeyCollector};

use super::memory::{HashSource, HashSourceBuilder};

/// File configuration source.
pub trait FileConfigSource: Send + Sync + Sized {
    /// Load source from string.
    fn load(content: &str) -> Result<Self, ConfigError>;

    /// Push value
    fn push_value(self, source: &mut HashSourceBuilder<'_>);

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
    #[inline]
    fn get_value(&self, key: &ConfigKey<'_>) -> Option<ConfigValue<'_>> {
        self.source.get_value(key)
    }

    #[inline]
    fn collect_keys<'a>(&'a self, prefix: &ConfigKey<'_>, sub: &mut PartialKeyCollector<'a>) {
        self.source.collect_keys(prefix, sub)
    }

    fn name(&self) -> &str {
        &self.name
    }

    #[inline]
    fn is_empty(&self) -> bool {
        self.source.is_empty()
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
            let value = S::load(&std::fs::read_to_string(path.clone())?)?;
            value.push_value(&mut source.prefixed());
        }
        Ok(source)
    }
}

/// File source.
#[doc(hidden)]
#[allow(missing_debug_implementations)]
pub struct InlineSource<S: FileConfigSource> {
    name: String,
    source: HashSource,
    _data: PhantomData<S>,
}

impl<S: FileConfigSource> InlineSource<S> {
    fn new(name: String, content: &str) -> Result<Self, ConfigError> {
        let value = S::load(content)?;
        let mut source = HashSource::new();
        value.push_value(&mut source.prefixed());
        Ok(Self {
            name,
            source,
            _data: PhantomData,
        })
    }
}

#[doc(hidden)]
#[inline]
pub fn inline_source<S: FileConfigSource>(
    name: String,
    content: &str,
) -> Result<InlineSource<S>, ConfigError> {
    InlineSource::new(name, content)
}

impl<S: FileConfigSource> ConfigSource for InlineSource<S> {
    fn name(&self) -> &str {
        &self.name
    }

    #[inline]
    fn get_value(&self, key: &ConfigKey<'_>) -> Option<ConfigValue<'_>> {
        self.source.get_value(key)
    }

    #[inline]
    fn collect_keys<'a>(&'a self, prefix: &ConfigKey<'_>, sub: &mut PartialKeyCollector<'a>) {
        self.source.collect_keys(prefix, sub)
    }

    fn is_empty(&self) -> bool {
        self.source.is_empty()
    }
}

/// Inline config source
#[doc(hidden)]
#[macro_export]
macro_rules! inline_config_source {
    ($ty:path: $path:literal) => {
        crate::source::file::inline_source::<$ty>(format!("inline:{}", $path), include_str!($path))
    };
}
