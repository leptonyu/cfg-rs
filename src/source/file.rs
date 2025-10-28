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
        let c = std::fs::read_to_string(&path)?;
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

#[cfg_attr(coverage_nightly, coverage(off))]
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
        assert!(!config.refreshable()?);
        update_file(&mut f)?;
        assert!(config.refreshable()?);
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
        assert!(!config.refresh()?);
        update_file(&mut f)?;
        assert!(config.refresh()?);
        std::fs::remove_file(path)?;
        Ok(())
    }

    #[test]
    fn inline_source_config_success() {
        // 使用 Temp 解析器，内容无所谓
        let result = super::inline_source_config::<Temp>("inline".to_string(), "abc");
        assert!(result.is_ok());
        let hs = result.unwrap();
        assert_eq!(hs.name(), "inline");
    }

    #[test]
    fn inline_source_config_parse_error() {
        struct Bad;
        impl ConfigSourceAdaptor for Bad {
            fn convert_source(self, _: &mut ConfigSourceBuilder<'_>) -> Result<(), ConfigError> {
                Ok(())
            }
        }
        impl ConfigSourceParser for Bad {
            type Adaptor = Bad;
            fn parse_source(_: &str) -> Result<Self::Adaptor, ConfigError> {
                Err(ConfigError::ConfigParseError(
                    "bad".to_string(),
                    "fail".to_string(),
                ))
            }
            fn file_extensions() -> Vec<&'static str> {
                vec!["bad"]
            }
        }
        let result = super::inline_source_config::<Bad>("bad".to_string(), "abc");
        assert!(matches!(result, Err(ConfigError::ConfigParseError(_, _))));
    }

    #[test]
    fn file_loader_load_required_not_exists() {
        // 测试 required=true 且文件不存在时返回 ConfigFileNotExists
        let path: PathBuf = "target/not_exist_file.tmp".into();
        let loader = <FileLoader<Temp>>::new(path.clone(), true, true);
        let mut hash_source = crate::source::memory::HashSource::new("test");
        let mut builder = hash_source.prefixed();
        let result = loader.load(&mut builder);
        assert!(matches!(result, Err(ConfigError::ConfigFileNotExists(p)) if p == path));
    }

    #[test]
    fn file_loader_load_ext_false_all_exts() {
        // 测试 ext=false 时会尝试所有扩展名
        let path: PathBuf = "target/file_multi_ext".into();
        // 创建一个带 .tmp 扩展名的文件
        let mut file_path = path.clone();
        file_path.set_extension("tmp");
        let mut f = File::create(&file_path).unwrap();
        f.write_all(b"abc").unwrap();
        f.flush().unwrap();

        let mut hash_source = crate::source::memory::HashSource::new("test");
        let mut builder = hash_source.prefixed();
        // 创建 loader 实例
        let loader = <FileLoader<Temp>>::new(path.clone(), true, false);
        // 应该能加载成功（flag 变为 false，不报错）
        let result = loader.load(&mut builder);
        assert!(result.is_ok());
        assert!(result.is_ok());

        std::fs::remove_file(file_path).unwrap();
    }
}
