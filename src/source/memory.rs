//! In memory source.
use std::collections::HashMap;

use crate::{
    key::{normalize_key, SubKey},
    ConfigKey, ConfigSource, ConfigValue, SubKeyList,
};

use super::collect_flattern_keys;

/// In memory source.
#[derive(Debug)]
pub struct MemorySource(String, HashMap<String, String>);

impl MemorySource {
    /// Create source.
    #[inline]
    pub fn new(name: String) -> Self {
        Self(name, HashMap::new())
    }

    /// Add config to source.
    #[inline]
    pub fn set<K: Into<String>, V: Into<String>>(mut self, k: K, v: V) -> Self {
        self.1.insert(normalize_key(&k.into()), v.into());
        self
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
        self.1.get(key.as_str()).map(|f| ConfigValue::StrRef(f))
    }

    #[inline]
    fn collect_keys<'a>(&'a self, prefix: &ConfigKey<'_>, sub: &mut SubKeyList<'a>) {
        collect_flattern_keys(self.1.keys().map(|f| f.as_str()), prefix, sub);
    }

    fn name(&self) -> &str {
        &self.0
    }
}

/// Memory Value.
#[derive(Debug)]
pub struct MemoryValue {
    pub(crate) array: Vec<MemoryValue>,
    pub(crate) table: HashMap<String, MemoryValue>,
    pub(crate) value: Option<ConfigValue<'static>>,
}

impl MemoryValue {
    pub(crate) fn new() -> Self {
        Self {
            array: vec![],
            table: HashMap::new(),
            value: None,
        }
    }

    fn sub_value(&self, key: &ConfigKey<'_>) -> Option<&Self> {
        let mut val = self;
        for n in key.iter() {
            match n {
                SubKey::Str(n) => val = val.table.get(*n)?,
                SubKey::Int(n) => val = val.array.get(*n)?,
            }
        }
        Some(val)
    }

    #[allow(dead_code)]
    pub(crate) fn with_prefix(&self, key: &str) -> Option<&Self> {
        let mut ck = ConfigKey::default();
        ck.push(key);
        self.sub_value(&ck)
    }

    pub(crate) fn get_value(&self, key: &ConfigKey<'_>) -> Option<ConfigValue<'_>> {
        let v = Self::sub_value(&self, key)?;
        let v = v.value.as_ref()?;
        Some(match v {
            ConfigValue::StrRef(v) => ConfigValue::StrRef(v),
            ConfigValue::Str(v) => ConfigValue::StrRef(&v),
            ConfigValue::Int(v) => ConfigValue::Int(*v),
            ConfigValue::Float(v) => ConfigValue::Float(*v),
            ConfigValue::Bool(v) => ConfigValue::Bool(*v),
        })
    }

    pub(crate) fn collect_keys<'a>(&'a self, prefix: &ConfigKey<'_>, sub: &mut SubKeyList<'a>) {
        if let Some(v) = Self::sub_value(self, prefix) {
            sub.insert_int(v.array.len());
            v.table.keys().for_each(|f| sub.insert_str(f));
        }
    }
}

use std::collections::HashSet;
pub struct HashSource(HashMap<String, HashValue>);
pub struct HashValue {
    sub_str: HashSet<String>,
    sub_int: Option<usize>,
    value: Option<ConfigValue<'static>>,
}

impl HashValue {
    fn new() -> Self {
        Self {
            sub_str: HashSet::new(),
            sub_int: None,
            value: None,
        }
    }

    fn push_val<V: Into<ConfigValue<'static>>>(&mut self, val: V) {
        self.value = Some(val.into());
    }

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

use crate::key::SubKeyIter;
impl HashSource {
    fn new() -> Self {
        Self(HashMap::new())
    }

    fn insert<V: Into<ConfigValue<'static>>>(&mut self, key: &str, value: V) {
        let iter: SubKeyIter<'_> = key.into();
        let mut key = "".to_string();
        for k in iter {
            let v = self
                .0
                .entry(key.clone())
                .or_insert_with(|| HashValue::new());
            v.push_key(&k);
            k.update_string(&mut key);
        }
        self.0
            .entry(key)
            .or_insert_with(|| HashValue::new())
            .push_val(value);
    }
}
