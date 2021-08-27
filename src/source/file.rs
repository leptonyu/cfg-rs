//! File config source.
use std::{marker::PhantomData, path::PathBuf};

use crate::ConfigError;

use super::{
    memory::{HashSource, HashSourceBuilder},
    Loader, SourceAdaptor, SourceLoader,
};

/// FileLoader
#[derive(Debug)]
pub(crate) struct FileLoader<L: SourceLoader> {
    name: String,
    path: PathBuf,
    ext: bool,
    required: bool,
    _data: PhantomData<L>,
}

impl<L: SourceLoader> FileLoader<L> {
    #[allow(dead_code)]
    pub(crate) fn new(path: PathBuf, required: bool, ext: bool) -> Self {
        Self {
            name: format!(
                "file:{}.[{}]",
                path.display(),
                L::file_extensions().join(",")
            ),
            path,
            ext,
            required,
            _data: PhantomData,
        }
    }
}

fn load_path<L: SourceLoader>(
    path: PathBuf,
    flag: &mut bool,
    builder: &mut HashSourceBuilder<'_>,
) -> Result<(), ConfigError> {
    if path.exists() {
        *flag = false;
        let c = std::fs::read_to_string(path)?;
        L::create_loader(&c)?.read_source(builder)?;
    }
    Ok(())
}

impl<L: SourceLoader> Loader for FileLoader<L> {
    fn name(&self) -> &str {
        &self.name
    }

    fn load(&self, builder: &mut HashSourceBuilder<'_>) -> Result<(), ConfigError> {
        let mut flag = self.required;
        if self.ext {
            load_path::<L>(self.path.clone(), &mut flag, builder)?;
        } else {
            for ext in L::file_extensions() {
                let mut path = self.path.clone();
                path.set_extension(ext);
                load_path::<L>(path, &mut flag, builder)?;
            }
        }
        if flag {
            return Err(ConfigError::ConfigFileNotExists(self.path.clone()));
        }
        Ok(())
    }
}

#[doc(hidden)]
#[inline]
pub fn inline_source<S: SourceLoader>(
    name: String,
    content: &'static str,
) -> Result<impl Loader, ConfigError> {
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
        crate::source::inline_source::<$ty>(format!("inline:{}", $path), include_str!($path))
    };
}
