use crate::{ConfigError, Configuration};

#[allow(unused_imports)]
use super::file::FileConfigSource;
use super::{
    memory::{HashSource, MemorySource},
    SourceType,
};

/// Read config content from network.
pub trait NetworkConfigReader {
    /// Read content.
    fn read_content(
        &self,
        name: &str,
        profile: Option<&str>,
        config: &Configuration,
    ) -> Result<Option<String>, ConfigError>;

    /// Source type.
    fn source_type(&self) -> SourceType;
}

impl dyn NetworkConfigReader {
    #[allow(unreachable_code, unused_mut)]
    pub(crate) fn load(
        &self,
        name: &str,
        profile: Option<&str>,
        config: &Configuration,
    ) -> Result<Option<MemorySource>, ConfigError> {
        if let Some(_content) = self.read_content(name, profile, config)? {
            let mut _source = HashSource::new();
            match self.source_type() {
                #[cfg(feature = "toml")]
                SourceType::Toml => {
                    super::toml::Toml::load(&_content)?.push_value(&mut _source.prefixed())
                }
                #[cfg(feature = "yaml")]
                SourceType::Yaml => {
                    super::yaml::Yaml::load(&_content)?.push_value(&mut _source.prefixed())
                }
                #[cfg(feature = "json")]
                SourceType::Json => {
                    super::json::Json::load(&_content)?.push_value(&mut _source.prefixed())
                }
            }
            return Ok(Some(_source.into_memory(name)));
        }
        Ok(None)
    }
}

#[cfg(test)]
#[cfg(feature = "toml")]
mod test {
    use crate::{source::SourceType, ConfigError, Configuration};

    use super::NetworkConfigReader;

    struct Noop;

    impl NetworkConfigReader for Noop {
        fn read_content(
            &self,
            _: &str,
            o: Option<&str>,
            _: &Configuration,
        ) -> Result<Option<String>, ConfigError> {
            Ok(Some(
                match o {
                    Some(_) => include_str!("../../ext-develop.toml"),
                    None => include_str!("../../ext.toml"),
                }
                .to_string(),
            ))
        }

        fn source_type(&self) -> SourceType {
            SourceType::Toml
        }
    }

    #[test]
    fn network_test() -> Result<(), ConfigError> {
        let config = Configuration::builder().set_network_source(Noop).init()?;
        assert_eq!("default", &config.get::<String>("external.profile")?);
        let config = Configuration::builder()
            .set("app.profile", "develop")
            .set_network_source(Noop)
            .init()?;
        assert_eq!("develop", &config.get::<String>("external.profile")?);
        Ok(())
    }
}
