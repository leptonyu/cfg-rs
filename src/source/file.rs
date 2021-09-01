//! File config source.
use std::{
    marker::PhantomData,
    path::{Path, PathBuf},
    time::SystemTime,
};

use crate::{err::ConfigLock, ConfigError, Mutex};

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
    modified: Mutex<Option<SystemTime>>,
    _data: PhantomData<L>,
}

fn modified_time(path: &Path) -> Option<SystemTime> {
    path.metadata().and_then(|a| a.modified()).ok()
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
            modified: Mutex::new(modified_time(&path)),
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

    fn allow_refresh(&self) -> bool {
        true
    }

    fn refreshable(&self) -> Result<bool, ConfigError> {
        let time = modified_time(&self.path);
        let mut g = self.modified.lock_c()?;
        let flag = time == *g;
        *g = time;
        Ok(!flag)
    }
}

#[doc(hidden)]
pub fn inline_source_config<S: ConfigSourceParser>(
    name: String,
    content: &'static str,
) -> Result<HashSource, ConfigError> {
    let v = S::parse_source(content)?;
    let mut m = HashSource::new(name);
    v.convert_source(&mut m.prefixed())?;
    Ok(m)
}

#[cfg(test)]
mod test {
    use std::{fs::File, io::Write, path::PathBuf};

    use crate::{
        source::{ConfigSource, ConfigSourceAdaptor, ConfigSourceBuilder, ConfigSourceParser},
        ConfigError, Configuration,
    };

    use super::FileLoader;

    struct Temp;

    impl ConfigSourceAdaptor for Temp {
        fn convert_source(self, _: &mut ConfigSourceBuilder<'_>) -> Result<(), ConfigError> {
            Ok(())
        }
    }
    impl ConfigSourceParser for Temp {
        type Adaptor = Temp;

        fn parse_source(_: &str) -> Result<Self::Adaptor, ConfigError> {
            Ok(Temp)
        }

        fn file_extensions() -> Vec<&'static str> {
            vec!["tmp"]
        }
    }

    #[test]
    fn refresh_file_test() -> Result<(), ConfigError> {
        let path: PathBuf = "target/file_2.tmp".into();
        let mut f = File::create(&path)?;
        let config = <FileLoader<Temp>>::new(path.clone(), false, true);
        assert_eq!(false, config.refreshable()?);
        update_file(&mut f)?;
        assert_eq!(true, config.refreshable()?);
        std::fs::remove_file(path)?;
        Ok(())
    }

    fn update_file(f: &mut File) -> Result<(), ConfigError> {
        let last = f.metadata()?.modified()?;
        let mut i = 0;
        while last == f.metadata()?.modified()? {
            i += 1;
            println!("Round: {}", i);
            f.write_all(b"hello")?;
            f.flush()?;
            std::thread::sleep(std::time::Duration::new(0, 1000000));
        }
        Ok(())
    }

    #[test]
    fn refresh_test() -> Result<(), ConfigError> {
        let path: PathBuf = "target/file.tmp".into();
        let mut f = File::create(&path)?;
        let mut config = Configuration::new().register_source(<FileLoader<Temp>>::new(
            path.clone(),
            false,
            true,
        ))?;
        assert_eq!(false, config.refresh()?);
        update_file(&mut f)?;
        assert_eq!(true, config.refresh()?);
        std::fs::remove_file(path)?;
        Ok(())
    }
}
