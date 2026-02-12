use crate::source::*;

pub use ::ini::Ini;
use ::ini::Properties;

impl ConfigSourceAdaptor for Properties {
    fn convert_source(self, builder: &mut ConfigSourceBuilder<'_>) -> Result<(), ConfigError> {
        for (k, v) in self.iter() {
            builder.set(k, v.to_string());
        }
        Ok(())
    }
}

impl ConfigSourceAdaptor for Ini {
    fn convert_source(self, builder: &mut ConfigSourceBuilder<'_>) -> Result<(), ConfigError> {
        let map = self.into_iter().filter_map(|(k, v)| k.map(|k| (k, v)));
        builder.insert_map(map)?;
        Ok(())
    }
}

impl ConfigSourceParser for Ini {
    type Adaptor = Ini;

    fn parse_source(c: &str) -> Result<Self::Adaptor, ConfigError> {
        Self::load_from_str(c).map_err(ConfigError::from_cause)
    }

    fn file_extensions() -> Vec<&'static str> {
        vec!["ini"]
    }
}

#[cfg_attr(coverage_nightly, coverage(off))]
#[cfg(test)]
mod test {
    use crate::{inline_source, test::source_test_suit, ConfigError};

    #[test]
    #[allow(unused_qualifications)]
    fn inline_test() -> Result<(), ConfigError> {
        source_test_suit(inline_source!("../../app.ini")?)
    }
}
