//! In memory source.
use std::{
    borrow::Borrow,
    collections::{HashMap, HashSet},
    vec,
};

use crate::{
    key::{PartialKey, PartialKeyIter},
    source::{ConfigSource, ConfigSourceAdaptor},
    value_ref::Refresher,
    ConfigError, ConfigKey, ConfigValue, PartialKeyCollector,
};

/// Hash Source.
#[doc(hidden)]
#[allow(missing_debug_implementations, unreachable_pub)]
pub struct HashSource {
    pub(crate) value: HashMap<String, HashValue>,
    name: String,
    pub(crate) refs: Refresher,
}

impl ConfigSource for HashSource {
    fn name(&self) -> &str {
        &self.name
    }
    fn load(&self, builder: &mut ConfigSourceBuilder<'_>) -> Result<(), ConfigError> {
        for (k, v) in &self.value {
            if let Some(v) = &v.value {
                builder.set(k, v.clone_static());
            }
        }
        Ok(())
    }
}

/// Hash Value.
#[derive(Debug)]
pub(crate) struct HashValue {
    sub_str: HashSet<String>,
    sub_int: Option<usize>,
    value: Option<ConfigValue<'static>>,
}

/// Config source builder.
#[derive(Debug)]
pub struct ConfigSourceBuilder<'a> {
    key: Vec<String>,
    map: &'a mut HashMap<String, HashValue>,
    count: usize,
}

impl HashValue {
    #[inline]
    fn new() -> Self {
        Self {
            sub_str: HashSet::new(),
            sub_int: None,
            value: None,
        }
    }

    #[inline]
    fn push_val<V: Into<ConfigValue<'static>>>(&mut self, val: V) {
        if self.value.is_none() {
            self.value = Some(val.into());
        }
    }

    #[inline]
    fn push_key(&mut self, key: &PartialKey<'_>) {
        match key {
            PartialKey::Str(i) => {
                self.sub_str.insert(i.to_string());
            }
            PartialKey::Int(i) => {
                let v = self.sub_int.get_or_insert(*i);
                if *v < *i {
                    *v = *i;
                }
            }
        }
    }
}

impl HashSource {
    pub(crate) fn new<K: Into<String>>(name: K) -> Self {
        Self {
            value: HashMap::new(),
            name: name.into(),
            refs: Refresher::new(),
        }
    }

    #[inline]
    pub(crate) fn prefixed(&mut self) -> ConfigSourceBuilder<'_> {
        ConfigSourceBuilder {
            key: vec![],
            map: &mut self.value,
            count: 0,
        }
    }

    pub(crate) fn get_value(&self, key: &ConfigKey<'_>) -> Option<ConfigValue<'_>> {
        let key = key.as_str();
        self.value
            .get(key)
            .and_then(|f| f.value.as_ref())
            .map(|v| match v {
                ConfigValue::StrRef(v) => ConfigValue::StrRef(v),
                ConfigValue::Str(v) => ConfigValue::StrRef(v),
                ConfigValue::Int(v) => ConfigValue::Int(*v),
                ConfigValue::Float(v) => ConfigValue::Float(*v),
                ConfigValue::Bool(v) => ConfigValue::Bool(*v),
                #[cfg(feature = "rand")]
                ConfigValue::Rand(v) => ConfigValue::Rand(*v),
            })
    }

    pub(crate) fn collect_keys<'a>(
        &'a self,
        prefix: &ConfigKey<'_>,
        sub: &mut PartialKeyCollector<'a>,
    ) {
        if let Some(v) = self.value.get(prefix.as_str()) {
            for k in v.sub_str.iter() {
                sub.str_key.insert(k.as_str());
            }
            if let Some(i) = v.sub_int {
                sub.insert_int(i);
            }
        }
    }
    pub(crate) fn set<K: Borrow<str>, V: Into<ConfigValue<'static>>>(mut self, k: K, v: V) -> Self {
        let mut c = self.prefixed();
        c.set(k.borrow(), v);
        self
    }
}

impl ConfigSourceBuilder<'_> {
    /// Set value.
    #[allow(single_use_lifetimes)]
    pub fn set<'b, K: Into<PartialKeyIter<'b>>, V: Into<ConfigValue<'static>>>(
        &mut self,
        k: K,
        v: V,
    ) {
        self.push(k);
        self.insert(v);
        self.pop();
    }

    /// Insert map into source.
    pub fn insert_map<I: IntoIterator<Item = (K, V)>, K: Borrow<str>, V: ConfigSourceAdaptor>(
        &mut self,
        iter: I,
    ) -> Result<(), ConfigError> {
        for (k, v) in iter {
            self.push(k.borrow());
            let x = v.convert_source(self);
            self.pop();
            x?;
        }
        Ok(())
    }

    /// Insert array into source.
    pub fn insert_array<I: IntoIterator<Item = S>, S: ConfigSourceAdaptor>(
        &mut self,
        iter: I,
    ) -> Result<(), ConfigError> {
        for (i, s) in iter.into_iter().enumerate() {
            self.push(i);
            let x = s.convert_source(self);
            self.pop();
            x?;
        }
        Ok(())
    }

    #[inline]
    fn push<'b, K: Into<PartialKeyIter<'b>>>(&mut self, key: K) {
        let mut curr = self.curr();
        let mut vs = vec![];
        let iter: PartialKeyIter<'b> = key.into();
        for k in iter {
            let v = self.map.entry(curr.clone()).or_insert_with(HashValue::new);
            v.push_key(&k);
            k.update_string(&mut curr);
            vs.push(k);
        }
        self.key.push(curr);
    }

    #[inline]
    fn pop(&mut self) {
        self.key.pop();
    }

    #[inline]
    fn curr(&self) -> String {
        self.key
            .last()
            .map(|f| f.as_str())
            .unwrap_or("")
            .to_string()
    }

    /// Insert value into source.
    #[inline]
    pub fn insert<V: Into<ConfigValue<'static>>>(&mut self, value: V) {
        self.count += 1;
        self.map
            .entry(self.curr())
            .or_insert_with(HashValue::new)
            .push_val(value);
    }

    pub(crate) fn count(&self) -> usize {
        self.count
    }
}

#[cfg_attr(coverage_nightly, coverage(off))]
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_source_new_and_name() {
        let hs = HashSource::new("abc");
        assert_eq!(hs.name(), "abc");
    }

    #[test]
    fn hash_source_prefixed_and_builder_set() {
        let mut hs = HashSource::new("test");
        {
            let mut builder = hs.prefixed();
            builder.set("a", 1);
            builder.set("b", "str");
        }
        let mut cache_a = crate::key::CacheString::new();
        let mut ka = cache_a.new_key();
        ka.push("a");
        let mut cache_b = crate::key::CacheString::new();
        let mut kb = cache_b.new_key();
        kb.push("b");
        match hs.get_value(&ka) {
            Some(ConfigValue::Int(1)) => {}
            _ => panic!("Expected Int(1)"),
        }
        match hs.get_value(&kb) {
            Some(ConfigValue::StrRef("str")) => {}
            Some(ConfigValue::Str(s)) if s == "str" => {}
            _ => panic!("Expected StrRef(\"str\")"),
        }
    }

    #[test]
    fn hash_source_collect_keys() {
        let mut hs = HashSource::new("test");
        {
            let mut builder = hs.prefixed();
            builder.set("foo.bar", 1);
            builder.set("foo.baz", 2);
            builder.set("foo[0]", 3);
        }
        let mut collector = PartialKeyCollector::new();
        let mut cache = crate::key::CacheString::new();
        let mut key = cache.new_key();
        key.push("foo");
        hs.collect_keys(&key, &mut collector);
        assert!(collector.str_key.iter().any(|&x| x == "bar"));
        assert!(collector.str_key.iter().any(|&x| x == "baz"));
        assert!(collector.int_key.iter().any(|&x| x == 1));
    }

    #[test]
    fn hash_value_push_key_and_val() {
        let mut hv = HashValue::new();
        hv.push_key(&PartialKey::Str("abc"));
        hv.push_key(&PartialKey::Int(2));
        hv.push_key(&PartialKey::Int(1));
        hv.push_val("val");
        hv.push_val("should_not_overwrite");
        assert!(hv.sub_str.contains("abc"));
        assert_eq!(hv.sub_int, Some(2));
        match hv.value {
            Some(ConfigValue::Str(ref s)) => assert_eq!(s, "val"),
            Some(ConfigValue::StrRef(s)) => assert_eq!(s, "val"),
            _ => panic!("Expected Str(\"val\")"),
        }
    }

    #[test]
    fn config_source_builder_insert_map_and_array() {
        struct Dummy;
        impl ConfigSourceAdaptor for Dummy {
            fn convert_source(
                self,
                builder: &mut ConfigSourceBuilder<'_>,
            ) -> Result<(), ConfigError> {
                builder.insert("dummy");
                Ok(())
            }
        }
        let mut hs = HashSource::new("test");
        {
            let mut builder = hs.prefixed();
            let map = vec![("k1", Dummy), ("k2", Dummy)];
            builder.insert_map(map).unwrap();
            let arr = vec![Dummy, Dummy];
            builder.insert_array(arr).unwrap();
        }
        assert!(hs.value.contains_key("k1"));
        assert!(hs.value.contains_key("k2"));
        assert!(hs.value.contains_key("[0]")); // index 1 of array
    }

    #[test]
    fn config_source_builder_push_pop_curr_count() {
        let mut hs = HashSource::new("test");
        let mut builder = hs.prefixed();
        assert_eq!(builder.curr(), "");
        builder.push("foo");
        assert_eq!(builder.curr(), "foo");
        builder.push("bar");
        assert_eq!(builder.curr(), "foo.bar");
        builder.pop();
        assert_eq!(builder.curr(), "foo");
        builder.pop();
        assert_eq!(builder.curr(), "");
        let c = builder.count();
        assert_eq!(c, 0);
    }

    #[test]
    fn config_source_load_sets_values() {
        let mut hs = HashSource::new("test");
        {
            let mut builder = hs.prefixed();
            builder.set("x", 42);
        }
        let mut target = HashSource::new("target");
        {
            let mut builder = target.prefixed();
            hs.load(&mut builder).unwrap();
        }
        let mut cache = crate::key::CacheString::new();
        let mut key = cache.new_key();
        key.push("x");
        match target.get_value(&key) {
            Some(ConfigValue::Int(42)) => {}
            _ => panic!("Expected Int(42)"),
        }
    }

    #[test]
    fn hash_source_get_value_variants() {
        let mut hs = HashSource::new("test");
        {
            let mut builder = hs.prefixed();
            builder.set("s1", "abc");
            builder.set("i1", 123);
            builder.set("f1", 3.14);
            builder.set("b1", true);
        }
        // String
        let mut cache = crate::key::CacheString::new();
        let mut key = cache.new_key();
        key.push("s1");
        match hs.get_value(&key) {
            Some(ConfigValue::StrRef("abc")) => {}
            Some(ConfigValue::Str(s)) if s == "abc" => {}
            _ => panic!("Expected StrRef(\"abc\")"),
        }
        // Int
        let mut cache = crate::key::CacheString::new();
        let mut key = cache.new_key();
        key.push("i1");
        match hs.get_value(&key) {
            Some(ConfigValue::Int(123)) => {}
            _ => panic!("Expected Int(123)"),
        }
        // Float
        let mut cache = crate::key::CacheString::new();
        let mut key = cache.new_key();
        key.push("f1");
        match hs.get_value(&key) {
            Some(ConfigValue::Float(f)) if (f - 3.14).abs() < 1e-6 => {}
            _ => panic!("Expected Float(3.14)"),
        }
        // Bool
        let mut cache = crate::key::CacheString::new();
        let mut key = cache.new_key();
        key.push("b1");
        match hs.get_value(&key) {
            Some(ConfigValue::Bool(true)) => {}
            _ => panic!("Expected Bool(true)"),
        }
        // Not found
        let mut cache = crate::key::CacheString::new();
        let mut key = cache.new_key();
        key.push("notfound");
        assert!(hs.get_value(&key).is_none());
    }
}
