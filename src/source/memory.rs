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
