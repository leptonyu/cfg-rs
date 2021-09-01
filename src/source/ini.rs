use crate::source::*;

use ::ini::ini::Properties;
pub use ::ini::Ini;

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
        builder.insert_map(self.into_iter().filter_map(|(k, v)| k.map(|k| (k, v))))?;
        Ok(())
    }
}

impl ConfigSourceParser for Ini {
    type Adaptor = Ini;

    fn parse_source(c: &str) -> Result<Self::Adaptor, ConfigError> {
        Ok(Self::load_from_str(c)?)
    }

    fn file_extensions() -> Vec<&'static str> {
        vec!["ini"]
    }
}

#[cfg(test)]
mod test {
    use crate::{inline_source, test::source_test_suit, ConfigError};

    #[test]
    #[allow(unused_qualifications)]
    fn inline_test() -> Result<(), ConfigError> {
        source_test_suit(inline_source!("../../app.ini")?)
    }
}
