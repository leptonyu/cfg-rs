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
