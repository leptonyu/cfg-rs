//! File config source.
use std::{marker::PhantomData, path::PathBuf};

use crate::ConfigError;

use super::{
    memory::{HashSourceBuilder, MemorySource},
    Loader, SourceAdaptor, SourceLoader,
};

/// File source loader, specify extenstions.
pub trait FileSourceLoader: SourceLoader {
    /// File extenstions.
    fn file_extensions() -> Vec<&'static str>;
}

/// FileLoader
#[derive(Debug)]
pub struct FileLoader<L: FileSourceLoader> {
    name: String,
    path: PathBuf,
    required: bool,
    _data: PhantomData<L>,
}

impl<L: FileSourceLoader> FileLoader<L> {
    pub(crate) fn new(path: PathBuf, required: bool) -> Self {
        Self {
            name: format!(
                "file:{}",
                path.as_path().as_os_str().to_str().expect("Not Possible")
            ),
            path,
            required,
            _data: PhantomData,
        }
    }
}

impl<L: FileSourceLoader> Loader for FileLoader<L> {
    fn load(&self, builder: &mut HashSourceBuilder<'_>) -> Result<(), ConfigError> {
        let mut flag = self.required;
        for ext in L::file_extensions() {
            let mut path = self.path.clone();
            path.set_extension(ext);
            if path.exists() {
                flag = false;
                let c = std::fs::read_to_string(path)?;
                L::create_loader(&c)?.load(builder)?
            }
        }
        if flag {
            return Err(ConfigError::ConfigFileNotExists("".to_string()));
        }
        Ok(())
    }
}

/// File configuration source.
pub trait FileConfigSource: Send + Sync + Sized {
    /// Load source from string.
    fn load(content: &str) -> Result<Self, ConfigError>;

    /// Push value
    fn push_value(self, source: &mut HashSourceBuilder<'_>);

    /// Configuration file extension.
    fn ext() -> &'static str;
}

#[doc(hidden)]
#[inline]
pub fn inline_source<S: SourceLoader>(
    name: String,
    content: &'static str,
) -> Result<MemorySource, ConfigError> {
    let v = S::create_loader(content)?;
    let mut m = MemorySource::new(name);
    v.load(&mut m.1.prefixed())?;
    Ok(m)
}

/// Inline config source
#[doc(hidden)]
#[macro_export]
macro_rules! inline_config_source {
    ($ty:path: $path:literal) => {
        crate::source::file::inline_source::<$ty>(format!("inline:{}", $path), include_str!($path))
    };
}
