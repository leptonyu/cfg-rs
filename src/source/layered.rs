//! Layered config source.

use crate::{ConfigKey, ConfigSource, ConfigValue};

/// Layered config source.
#[allow(missing_debug_implementations)]
pub struct LayeredSource {
    layer: Vec<Box<dyn ConfigSource + 'static>>,
}

impl LayeredSource {
    /// Create new source.
    pub fn new() -> Self {
        LayeredSource { layer: vec![] }
    }

    /// Register source.
    #[inline]
    pub fn register(&mut self, source: impl ConfigSource + 'static) {
        self.layer.push(Box::new(source));
    }

    /// Get source names.
    pub fn source_names(&self) -> Vec<&str> {
        self.layer.iter().map(|f| f.name()).collect()
    }
}

impl ConfigSource for LayeredSource {
    fn get_value(&self, key: &ConfigKey<'_>) -> Option<ConfigValue<'_>> {
        for s in self.layer.iter() {
            if let Some(v) = s.as_ref().get_value(key) {
                return Some(v);
            }
        }
        None
    }

    #[inline]
    fn collect_keys<'a>(&'a self, prefix: &ConfigKey<'_>, sub: &mut crate::SubKeyList<'a>) {
        for s in &self.layer {
            s.collect_keys(prefix, sub);
        }
    }

    fn name(&self) -> &str {
        "layered_source"
    }
}
