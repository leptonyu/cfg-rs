use std::{cell::RefCell, collections::HashSet};

use crate::{impl_cache, ConfigError};

/// Config key, [ConfigSource](source/trait.ConfigSource.html) use this key to access config properties.
///
/// It's designed for better querying sources.
///
/// Config key has a normalized string representation, it is composed by
/// multiple partial keys, which are [`&str`] or [`usize`]. We use dot(`.`) and
/// square bracket `[]` to separate partial keys in a config key.
///
/// # Partial Key
///
/// ## String Partial Key
///
/// String partial key has regex pattern `[a-z][_a-z0-9]`, usually string partial keys are separated by dot(`.`).
/// For example: `cfg.k1`.
///
/// ## Index Partial Key
///
/// Index partial keys are [`usize`] values, which is around by a pair of square bracket.
/// For example: `[0]`, `[1]`.
///
/// # Config Key
///
/// Config key is composed by partial keys. String partial key can be followed by index partial key for zero or more times.
/// If the string config key is not in head, then it should after a dot(`.`).
/// For example:
///
///   * `cfg.v1`
///   * `cfg.v2[0]`
///   * `cfg.v3[0][1]`
///   * `cfg.v4.key`
///   * `cfg.v5.arr[0]`
///   * `[0]`
///
/// Please notice that `cfg.[0]` is invalid key.
///
pub type ConfigKey<'a> = CacheKey<'a>;

#[derive(Debug)]
pub(crate) struct CacheString {
    current: String,
    mark: Vec<(usize, usize)>,
}
thread_local! {
    static BUG: RefCell<CacheString> = RefCell::new(CacheString::new());
}
impl CacheString {
    pub(crate) fn new() -> CacheString {
        Self {
            current: String::with_capacity(10),
            mark: Vec::with_capacity(3),
        }
    }

    #[inline]
    #[allow(single_use_lifetimes)]
    fn push<'a, I: IntoIterator<Item = PartialKey<'a>>>(&mut self, iter: I) {
        let mut step = 0;
        let len = self.current.len();
        for i in iter {
            step += 1;
            i.update_string(&mut self.current);
        }
        self.mark.push((step, len));
    }

    #[inline]
    fn pop(&mut self) {
        if let Some((s, l)) = self.mark.pop() {
            if s > 0 {
                self.current.truncate(l);
            }
        }
    }

    #[inline]
    fn clear(&mut self) {
        self.current.clear();
        self.mark.clear();
    }

    #[inline]
    pub(crate) fn new_key(&mut self) -> CacheKey<'_> {
        CacheKey { cache: self }
    }

    #[inline]
    pub(crate) fn with_key_place<T, F: FnMut(&mut Self) -> Result<T, ConfigError>>(
        f: F,
    ) -> Result<T, ConfigError> {
        BUG.with(move |buf| Self::with_key_buf(buf, f))
    }
}

impl_cache!(CacheString);

/// The implementation of [`ConfigKey`].
#[derive(Debug)]
pub struct CacheKey<'a> {
    cache: &'a mut CacheString,
}

impl Drop for CacheKey<'_> {
    fn drop(&mut self) {
        self.cache.clear();
    }
}

impl<'a> CacheKey<'a> {
    pub(crate) fn push<I: Into<PartialKeyIter<'a>>>(&mut self, iter: I) {
        self.cache.push(iter.into());
    }
    pub(crate) fn pop(&mut self) {
        self.cache.pop();
    }

    /// As string
    pub(crate) fn as_str(&self) -> &str {
        &self.cache.current
    }
}
impl std::fmt::Display for CacheKey<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}
/// Partial key, plese refer to [`ConfigKey`].
#[allow(single_use_lifetimes)]
#[derive(Debug, PartialEq, Eq)]
pub enum PartialKey<'a> {
    /// String Partial Key.
    Str(&'a str),
    /// Index Partial Key.
    Int(usize),
}

impl PartialKey<'_> {
    #[inline]
    pub(crate) fn update_string(&self, key_long: &mut String) {
        match self {
            PartialKey::Int(i) => {
                key_long.push('[');
                key_long.push_str(&i.to_string());
                key_long.push(']');
            }
            PartialKey::Str(v) => {
                if !key_long.is_empty() {
                    key_long.push('.');
                }
                key_long.push_str(v);
            }
        }
    }
}

#[derive(Debug)]
#[allow(variant_size_differences)]
pub enum PartialKeyIter<'a> {
    Str(std::str::Split<'a, &'a [char]>),
    Int(Option<usize>),
}

impl<'a> Iterator for PartialKeyIter<'a> {
    type Item = PartialKey<'a>;
    fn next(&mut self) -> Option<Self::Item> {
        match self {
            PartialKeyIter::Str(s) => {
                for v in s {
                    if v.is_empty() {
                        continue;
                    }
                    return Some(if let Ok(i) = v.parse() {
                        PartialKey::Int(i)
                    } else {
                        PartialKey::Str(v)
                    });
                }
                None
            }
            PartialKeyIter::Int(x) => x.take().map(|x| x.into()),
        }
    }
}

/// Partial key collector.
#[derive(Debug)]
pub struct PartialKeyCollector<'a> {
    pub(crate) str_key: HashSet<&'a str>,
    pub(crate) int_key: Option<usize>,
}

#[allow(single_use_lifetimes)]
impl<'a> PartialKeyCollector<'a> {
    pub(crate) fn new() -> Self {
        Self {
            str_key: HashSet::new(),
            int_key: None,
        }
    }

    // #[allow(dead_code)]
    // /// Add string key.
    // pub(crate) fn insert_str(&mut self, key: &'a str) {
    //     if let Ok(i) = key.parse() {
    //         self.insert_int(i);
    //     } else {
    //         self.str_key.insert(key);
    //     }
    // }

    /// Add index of array.
    pub(crate) fn insert_int(&mut self, key: usize) {
        if let Some(u) = self.int_key {
            if u > key {
                return;
            }
        }
        self.int_key = Some(key + 1);
    }
}

// impl<'a> Into<PartialKey<'a>> for &'a str {
//     fn into(self) -> PartialKey<'a> {
//         PartialKey::Str(self)
//     }
// }

impl<'a> From<&'a str> for PartialKeyIter<'a> {
    fn from(s: &'a str) -> Self {
        PartialKeyIter::Str(s.split(&['.', '[', ']'][..]))
    }
}

impl From<usize> for PartialKey<'_> {
    fn from(s: usize) -> Self {
        PartialKey::Int(s)
    }
}

impl From<usize> for PartialKeyIter<'_> {
    fn from(s: usize) -> Self {
        PartialKeyIter::Int(Some(s))
    }
}

impl<'a> From<&'a String> for PartialKeyIter<'a> {
    fn from(s: &'a String) -> Self {
        s.as_str().into()
    }
}

#[cfg(test)]
mod test {
    use super::*;

    macro_rules! should_eq {
        ($origin:expr => $norm:expr) => {
            let mut che = CacheString::new();
            let mut key = che.new_key();
            key.push($origin);
            assert_eq!(&key.to_string(), $norm);
            let mut chd = CacheString::new();
            let mut kez = chd.new_key();
            kez.push($norm);
            assert_eq!(true, key.as_str() == kez.as_str());
        };
    }

    #[test]
    fn key_test() {
        should_eq!("" => "");
        should_eq!("." => "");
        should_eq!(".." => "");
        should_eq!("[" => "");
        should_eq!("[1]" => "[1]");
        should_eq!("1" => "[1]");
        should_eq!("1[1]" => "[1][1]");
        should_eq!("prefix.prop" => "prefix.prop");
        should_eq!(".prefix.prop"=> "prefix.prop");
        should_eq!("[]prefix.prop"=> "prefix.prop");
        should_eq!("[0]prefix.prop"=> "[0].prefix.prop");
        should_eq!("prefix[0].prop"=> "prefix[0].prop");
        should_eq!("prefix.0.prop"=> "prefix[0].prop");
        should_eq!("hello" => "hello");
    }

    macro_rules! should_ls {
        ($($origin:literal => $norm:literal,)+) => {
            let mut che = CacheString::new();
            let mut key = che.new_key();
            let mut vec = vec![];
            $(
                key.push($origin);
                assert_eq!($norm, key.as_str());
                vec.push(key.to_string());
            )+
            while let Some(v) = vec.pop() {
                assert_eq!(&v, key.as_str());
                key.pop();
            }
        };
    }

    #[test]
    fn key_push_test() {
        should_ls!(
            "a" => "a",
            "" => "a",
            "b" => "a.b",
            "1" => "a.b[1]",
            "1" => "a.b[1][1]",
            "a.1" => "a.b[1][1].a[1]",
        );
    }
}
