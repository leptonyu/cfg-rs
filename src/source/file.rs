//! File config source.
use std::{marker::PhantomData, path::PathBuf};

use crate::ConfigError;

use super::{
    memory::{HashSource, HashSourceBuilder},
    Loader, SourceAdaptor, SourceLoader,
};

/// FileLoader
#[derive(Debug)]
pub struct FileLoader<L: SourceLoader> {
    name: String,
    path: PathBuf,
    required: bool,
    _data: PhantomData<L>,
}

impl<L: SourceLoader> FileLoader<L> {
    #[allow(dead_code)]
    pub(crate) fn new(path: PathBuf, required: bool) -> Self {
        Self {
            name: format!(
                "file:{}.[{}]",
                path.as_path().as_os_str().to_str().expect("Not Possible"),
                L::file_extensions().join(",")
            ),
            path,
            required,
            _data: PhantomData,
        }
    }
}

impl<L: SourceLoader> Loader for FileLoader<L> {
    fn name(&self) -> &str {
        &self.name
    }

    fn load(&self, builder: &mut HashSourceBuilder<'_>) -> Result<(), ConfigError> {
        let mut flag = self.required;
        for ext in L::file_extensions() {
            let mut path = self.path.clone();
            path.set_extension(ext);
            if path.exists() {
                flag = false;
                let c = std::fs::read_to_string(path)?;
                L::create_loader(&c)?.read_source(builder)?
            }
        }
        if flag {
            return Err(ConfigError::ConfigFileNotExists("".to_string()));
        }
        Ok(())
    }
}

#[doc(hidden)]
#[inline]
pub fn inline_source<S: SourceLoader>(
    name: String,
    content: &'static str,
) -> Result<HashSource, ConfigError> {
    let v = S::create_loader(content)?;
    let mut m = HashSource::new(name);
    v.read_source(&mut m.prefixed())?;
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
