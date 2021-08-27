//! File config source.
use std::{marker::PhantomData, path::PathBuf};

use crate::ConfigError;

use super::{
    memory::{ConfigSourceBuilder, HashSource},
    ConfigSource, ConfigSourceAdaptor, ConfigSourceParser,
};

/// FileLoader
#[derive(Debug)]
pub(crate) struct FileLoader<L: ConfigSourceParser> {
    name: String,
    path: PathBuf,
    ext: bool,
    required: bool,
    _data: PhantomData<L>,
}

impl<L: ConfigSourceParser> FileLoader<L> {
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

fn load_path<L: ConfigSourceParser>(
    path: PathBuf,
    flag: &mut bool,
    builder: &mut ConfigSourceBuilder<'_>,
) -> Result<(), ConfigError> {
    if path.exists() {
        *flag = false;
        let c = std::fs::read_to_string(path)?;
        L::parse_source(&c)?.convert_source(builder)?;
    }
    Ok(())
}

impl<L: ConfigSourceParser> ConfigSource for FileLoader<L> {
    fn name(&self) -> &str {
        &self.name
    }

    fn load(&self, builder: &mut ConfigSourceBuilder<'_>) -> Result<(), ConfigError> {
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
pub fn inline_source<S: ConfigSourceParser>(
    name: String,
    content: &'static str,
) -> Result<impl ConfigSource, ConfigError> {
    let v = S::parse_source(content)?;
    let mut m = HashSource::new(name);
    v.convert_source(&mut m.prefixed())?;
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
