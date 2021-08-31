use crate::{
    err::ConfigLock,
    source::{memory::HashSource, ConfigSource, ConfigSourceBuilder},
    ConfigError, Mutex,
};

#[macro_export]
#[doc(hidden)]
macro_rules! impl_cache {
    ($x:ident) => {
        thread_local! {
            static BUF: RefCell<$x> = RefCell::new($x::new());
        }
        impl $x {
            #[inline]
            #[allow(dead_code)]
            pub(crate) fn with_key<T, F: FnMut(&mut Self) -> Result<T, ConfigError>>(
                f: F,
            ) -> Result<T, ConfigError> {
                BUF.with(move |buf| Self::with_key_buf(buf, f))
            }

            #[allow(dead_code)]
            fn with_key_buf<T, F: FnMut(&mut Self) -> Result<T, ConfigError>>(
                buf: &RefCell<$x>,
                mut f: F,
            ) -> Result<T, ConfigError> {
                let borrow = buf.try_borrow_mut();
                let mut a;
                let mut b;
                let buf = match borrow {
                    Ok(buf) => {
                        a = buf;
                        &mut *a
                    }
                    _ => {
                        b = $x::new();
                        &mut b
                    }
                };
                (f)(buf)
            }
        }
    };
}

/// Cacheable source.
pub(crate) struct CacheConfigSource<L: ConfigSource> {
    cache: Mutex<(Option<HashSource>, bool)>,
    origin: L,
}

impl<L: ConfigSource> CacheConfigSource<L> {
    pub(crate) fn new(origin: L) -> Self {
        Self {
            cache: Mutex::new((None, false)),
            origin,
        }
    }
}

impl<L: ConfigSource> ConfigSource for CacheConfigSource<L> {
    fn name(&self) -> &str {
        self.origin.name()
    }

    fn load(&self, builder: &mut ConfigSourceBuilder<'_>) -> Result<(), ConfigError> {
        let mut g = self.cache.lock_c()?;
        if g.1 || g.0.is_none() {
            let mut source = HashSource::new(format!("cache:{}", self.origin.name()));
            self.origin.load(&mut source.prefixed())?;
            *g = (Some(source), false);
        }
        g.0.as_ref().expect("NP").load(builder)
    }

    fn allow_refresh(&self) -> bool {
        self.origin.allow_refresh()
    }

    fn refreshable(&self) -> Result<bool, ConfigError> {
        if !self.allow_refresh() {
            return Ok(false);
        }
        let flag = self.origin.refreshable()?;
        self.cache.lock_c()?.1 = flag;
        Ok(flag)
    }
}
