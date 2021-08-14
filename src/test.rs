use crate::*;

pub(crate) trait TestConfigExt: ConfigSource + Sized + 'static {
    fn new_config(self) -> Configuration {
        Configuration::new().register_source(self)
    }
}

impl<C: ConfigSource + 'static> TestConfigExt for C {}
