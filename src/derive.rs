use crate::FromConfig;

/// Config with prefix. This trait is auto derived by [FromConfig](./derive.FromConfig.html#struct-annotation-attribute).
pub trait FromConfigWithPrefix: FromConfig {
    /// Predefined key of config, so you don't have to provide it.
    fn prefix() -> &'static str;
}

#[cfg_attr(coverage_nightly, coverage(off))]
#[cfg(test)]
mod test {
    use crate::{source::memory::HashSource, test::TestConfigExt, *};
    #[derive(FromConfig, Debug, PartialEq, Eq)]
    #[config(prefix = "app", crate = "crate")]
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
        let config = HashSource::new("test")
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
