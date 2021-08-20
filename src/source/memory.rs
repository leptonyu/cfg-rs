//! In memory source.
use std::{
    borrow::Borrow,
    collections::{HashMap, HashSet},
    vec,
};

use crate::{
    key::{SubKey, SubKeyIter},
    source::file::FileConfigSource,
    ConfigKey, ConfigSource, ConfigValue, SubKeyList,
};

/// In memory source.
#[derive(Debug)]
pub struct MemorySource(String, HashSource);

impl MemorySource {
    /// Create source.
    #[inline]
    pub fn new(name: String) -> Self {
        Self(name, HashSource::new())
    }

    /// Add config to source.
    #[inline]
    #[allow(single_use_lifetimes)]
    pub fn set<K: Borrow<str>, V: Into<ConfigValue<'static>>>(mut self, k: K, v: V) -> Self {
        self.insert(k, v);
        self
    }

    /// Add config to source.
    #[inline]
    #[allow(single_use_lifetimes)]
    pub(crate) fn insert<K: Borrow<str>, V: Into<ConfigValue<'static>>>(&mut self, k: K, v: V) {
        let mut source = self.1.prefixed();
        source.push(k.borrow());
        source.insert(v);
        source.pop();
    }
}

impl Default for MemorySource {
    fn default() -> Self {
        MemorySource::new("default".to_string())
    }
}

impl ConfigSource for MemorySource {
    #[inline]
    fn get_value(&self, key: &ConfigKey<'_>) -> Option<ConfigValue<'_>> {
        self.1.get_value(key)
    }

    #[inline]
    fn collect_keys<'a>(&'a self, prefix: &ConfigKey<'_>, sub: &mut SubKeyList<'a>) {
        self.1.collect_keys(prefix, sub)
    }

    fn name(&self) -> &str {
        &self.0
    }

    fn is_empty(&self) -> bool {
        self.1.is_empty()
    }
}

/// Hash Source.
#[derive(Debug)]
pub(crate) struct HashSource(HashMap<String, HashValue>);

/// Hash Value.
#[derive(Debug)]
pub(crate) struct HashValue {
    sub_str: HashSet<String>,
    sub_int: Option<usize>,
    value: Option<ConfigValue<'static>>,
}

/// Prefixed hash source.
#[derive(Debug)]
pub struct HashSourceBuilder<'a> {
    key: Vec<String>,
    map: &'a mut HashMap<String, HashValue>,
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
        self.value = Some(val.into());
    }

    #[inline]
    fn push_key(&mut self, key: &SubKey<'_>) {
        match key {
            SubKey::Str(i) => {
                self.sub_str.insert(i.to_string());
            }
            SubKey::Int(i) => {
                let v = self.sub_int.get_or_insert(*i);
                if *v < *i {
                    *v = *i;
                }
            }
        }
    }
}

impl HashSource {
    pub(crate) fn new() -> Self {
        Self(HashMap::new())
    }

    #[inline]
    pub(crate) fn prefixed(&mut self) -> HashSourceBuilder<'_> {
        HashSourceBuilder {
            key: vec![],
            map: &mut self.0,
        }
    }

    pub(crate) fn get_value(&self, key: &ConfigKey<'_>) -> Option<ConfigValue<'_>> {
        let key = key.as_str();
        self.0
            .get(key)
            .and_then(|f| f.value.as_ref())
            .map(|v| match v {
                ConfigValue::StrRef(v) => ConfigValue::StrRef(v),
                ConfigValue::Str(v) => ConfigValue::StrRef(&v),
                ConfigValue::Int(v) => ConfigValue::Int(*v),
                ConfigValue::Float(v) => ConfigValue::Float(*v),
                ConfigValue::Bool(v) => ConfigValue::Bool(*v),
            })
    }

    pub(crate) fn collect_keys<'a>(&'a self, prefix: &ConfigKey<'_>, sub: &mut SubKeyList<'a>) {
        if let Some(v) = self.0.get(prefix.as_str()) {
            for k in v.sub_str.iter() {
                sub.str_key.insert(k.as_str());
            }
            if let Some(i) = v.sub_int {
                sub.insert_int(i);
            }
        }
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl HashSourceBuilder<'_> {
    /// Insert map into source.
    pub fn insert_map<I: IntoIterator<Item = (K, V)>, K: Borrow<str>, V: FileConfigSource>(
        &mut self,
        iter: I,
    ) {
        for (k, v) in iter {
            self.push(k.borrow());
            v.push_value(self);
            self.pop();
        }
    }

    /// Insert array into source.
    pub fn insert_array<I: IntoIterator<Item = S>, S: FileConfigSource>(&mut self, iter: I) {
        let mut i = 0;
        for s in iter {
            self.push(i);
            i += 1;
            s.push_value(self);
            self.pop();
        }
    }

    #[inline]
    fn push<'b, K: Into<SubKeyIter<'b>>>(&mut self, key: K) {
        let mut curr = self.curr();
        let mut vs = vec![];
        let iter: SubKeyIter<'b> = key.into();
        for k in iter {
            let v = self
                .map
                .entry(curr.clone())
                .or_insert_with(|| HashValue::new());
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
        self.map
            .entry(self.curr())
            .or_insert_with(|| HashValue::new())
            .push_val(value);
    }
}
