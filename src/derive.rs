use crate::FromConfig;

/// Configuration with prefix, used for auto derive.
pub trait FromConfigWithPrefix: FromConfig {
    /// Prefix of config.
    fn prefix() -> &'static str;
}

#[cfg(test)]
mod test {
    use crate::{source::memory::MemorySource, test::TestConfigExt, *};
    #[derive(FromConfig, Debug, PartialEq, Eq)]
    #[config(prefix = "app")]
    pub(crate) struct ConfigObject {
        hello: String,
        option: Option<String>,
        list: Vec<String>,
        count: u8,
        #[config(name = "count")]
        count_rename: u8,
        #[config(default = 3)]
        def: u8,
    }

    #[test]
    fn derive_test() {
        let config = MemorySource::default()
            .set("app.hello", "world")
            .set("app.count", "1")
            .set("app.count_rename", "2")
            .new_config();
        let object: ConfigObject = config.get("app").unwrap();
        assert_eq!("world", object.hello);
        assert_eq!(None, object.option);
        let v: Vec<String> = vec![];
        assert_eq!(v, object.list);
        assert_eq!(1, object.count);
        assert_eq!(1, object.count_rename);
        assert_eq!(3, object.def);
        let object2: ConfigObject = config.get_predefined().unwrap();
        assert_eq!(object, object2);
    }
}
