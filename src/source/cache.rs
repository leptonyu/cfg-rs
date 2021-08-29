use std::sync::Mutex;

use crate::{err::ConfigLock, ConfigError};

use super::{memory::HashSource, ConfigSource, ConfigSourceBuilder};

/// Cacheable source.
pub(crate) struct CacheConfigSource<L: ConfigSource> {
    cache: Mutex<Option<HashSource>>,
    origin: L,
}

impl<L: ConfigSource> CacheConfigSource<L> {
    pub(crate) fn new(origin: L) -> Self {
        Self {
            cache: Mutex::new(None),
            origin,
        }
    }
}

impl<L: ConfigSource> ConfigSource for CacheConfigSource<L> {
    fn name(&self) -> &str {
        self.origin.name()
    }

    fn load(&self, builder: &mut ConfigSourceBuilder<'_>) -> Result<(), ConfigError> {
        let flag = self.origin.refreshable()?;
        let mut g = self.cache.lock_c()?;
        if flag || g.is_none() {
            let mut source = HashSource::new(format!("cache:{}", self.origin.name()));
            self.origin.load(&mut source.prefixed())?;
            *g = Some(source);
        }
        g.as_ref().expect("NP").load(builder)
    }
}
