//! In memory source.
use std::{
    borrow::Borrow,
    collections::{HashMap, HashSet},
    vec,
};

use crate::{
    key::{PartialKey, PartialKeyIter},
    source::{Loader, SourceAdaptor},
    ConfigError, ConfigKey, ConfigValue, PartialKeyCollector,
};

/// In memory source.
#[derive(Debug)]
pub struct MemorySource(String, pub(crate) HashSource);

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
        source.set(k.borrow(), v);
    }
}

impl Default for MemorySource {
    fn default() -> Self {
        MemorySource::new("default".to_string())
    }
}

impl Loader for MemorySource {
    fn load(&self, builder: &mut HashSourceBuilder<'_>) -> Result<(), ConfigError> {
        for (k, v) in &self.1 .0 {
            if let Some(v) = &v.value {
                builder.set(k, v.clone_static());
            }
        }
        Ok(())
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
                #[cfg(feature = "rand")]
                ConfigValue::Rand(v) => ConfigValue::Rand(*v),
            })
    }

    pub(crate) fn collect_keys<'a>(
        &'a self,
        prefix: &ConfigKey<'_>,
        sub: &mut PartialKeyCollector<'a>,
    ) {
        if let Some(v) = self.0.get(prefix.as_str()) {
            for k in v.sub_str.iter() {
                sub.str_key.insert(k.as_str());
            }
            if let Some(i) = v.sub_int {
                sub.insert_int(i);
            }
        }
    }
}

impl HashSourceBuilder<'_> {
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
    pub fn insert_map<I: IntoIterator<Item = (K, V)>, K: Borrow<str>, V: SourceAdaptor>(
        &mut self,
        iter: I,
    ) -> Result<(), ConfigError> {
        for (k, v) in iter {
            self.push(k.borrow());
            let x = v.load(self);
            self.pop();
            x?;
        }
        Ok(())
    }

    /// Insert array into source.
    pub fn insert_array<I: IntoIterator<Item = S>, S: SourceAdaptor>(
        &mut self,
        iter: I,
    ) -> Result<(), ConfigError> {
        let mut i = 0;
        for s in iter {
            self.push(i);
            i += 1;
            let x = s.load(self);
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
