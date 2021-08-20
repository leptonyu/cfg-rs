use std::{cell::RefCell, collections::HashSet, slice::Iter};

use crate::ConfigError;

///Config Values
pub type ConfigKey<'a> = CacheKey<'a>;
// pub type ConfigKey<'a> = HashKey<'a>;

#[derive(Debug)]
pub(crate) struct CacheString {
    current: String,
    mark: Vec<(usize, usize)>,
}
thread_local! {
    static BUF: RefCell<CacheString> = RefCell::new(CacheString::new());
}
impl CacheString {
    pub(crate) fn new() -> CacheString {
        Self {
            current: String::with_capacity(10),
            mark: Vec::with_capacity(5),
        }
    }

    fn push<'a, I: IntoIterator<Item = SubKey<'a>>>(
        &mut self,
        iter: I,
        keys: &mut Vec<SubKey<'a>>,
    ) {
        let mut step = 0;
        let len = self.current.len();
        for i in iter {
            step += 1;
            i.update_string(&mut self.current);
            keys.push(i);
        }
        self.mark.push((step, len));
    }

    fn pop(&mut self, keys: &mut Vec<SubKey<'_>>) {
        if let Some((s, l)) = self.mark.pop() {
            if s > 0 {
                keys.drain(keys.len() - s..);
                self.current.truncate(l);
            }
        }
    }

    fn clear(&mut self) {
        self.current.clear();
        self.mark.clear();
    }

    pub(crate) fn new_key(&mut self) -> CacheKey<'_> {
        CacheKey {
            cache: self,
            keys: Vec::with_capacity(5),
        }
    }

    pub(crate) fn with_key<T, F: Fn(&mut Self) -> Result<T, ConfigError>>(
        f: F,
    ) -> Result<T, ConfigError> {
        BUF.with(move |buf| {
            let borrow = buf.try_borrow_mut();
            let mut a;
            let mut b;
            let buf = match borrow {
                Ok(buf) => {
                    a = buf;
                    &mut *a
                }
                _ => {
                    b = CacheString::new();
                    &mut b
                }
            };
            (f)(buf)
        })
    }
}

#[derive(Debug)]
pub struct CacheKey<'a> {
    cache: &'a mut CacheString,
    keys: Vec<SubKey<'a>>,
}

impl Drop for CacheKey<'_> {
    fn drop(&mut self) {
        self.cache.clear();
    }
}

impl<'a> CacheKey<'a> {
    pub(crate) fn push<I: Into<SubKeyIter<'a>>>(&mut self, iter: I) {
        self.cache.push(iter.into(), &mut self.keys);
    }
    pub(crate) fn pop(&mut self) {
        self.cache.pop(&mut self.keys);
    }

    #[allow(dead_code)]
    fn iter(&self) -> Iter<'_, SubKey<'_>> {
        self.keys.iter()
    }

    /// As string
    pub(crate) fn as_str(&self) -> &str {
        &self.cache.current
    }

    /// To String.
    pub(crate) fn to_string(&self) -> String {
        self.as_str().to_string()
    }
}

#[allow(single_use_lifetimes)]
#[derive(Debug, PartialEq, Eq)]
pub enum SubKey<'a> {
    Str(&'a str),
    Int(usize),
}

impl SubKey<'_> {
    #[inline]
    pub(crate) fn update_string(&self, key_long: &mut String) {
        match self {
            SubKey::Int(i) => {
                key_long.push('[');
                key_long.push_str(&i.to_string());
                key_long.push(']');
            }
            SubKey::Str(v) => {
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
pub enum SubKeyIter<'a> {
    Str(std::str::Split<'a, &'a [char]>),
    Int(Option<usize>),
}

impl<'a> Iterator for SubKeyIter<'a> {
    type Item = SubKey<'a>;
    fn next(&mut self) -> Option<Self::Item> {
        match self {
            SubKeyIter::Str(s) => {
                while let Some(v) = s.next() {
                    if v.is_empty() {
                        continue;
                    }
                    return Some(if let Ok(i) = v.parse() {
                        SubKey::Int(i)
                    } else {
                        SubKey::Str(v)
                    });
                }
                None
            }
            SubKeyIter::Int(x) => x.take().map(|x| x.into()),
        }
    }
}

#[doc(hidden)]
/// Sub key list.
#[derive(Debug)]
pub struct SubKeyList<'a> {
    pub(crate) str_key: HashSet<&'a str>,
    pub(crate) int_key: Option<usize>,
}

impl<'a> SubKeyList<'a> {
    pub(crate) fn new() -> Self {
        Self {
            str_key: HashSet::new(),
            int_key: None,
        }
    }

    #[allow(dead_code)]
    /// Add string key.
    pub(crate) fn insert_str(&mut self, key: &'a str) {
        if let Ok(i) = key.parse() {
            self.insert_int(i);
        } else {
            self.str_key.insert(key);
        }
    }

    #[allow(dead_code)]
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

impl<'a> Into<SubKey<'a>> for &'a str {
    fn into(self) -> SubKey<'a> {
        SubKey::Str(self)
    }
}
impl<'a> Into<SubKeyIter<'a>> for &'a str {
    fn into(self) -> SubKeyIter<'a> {
        SubKeyIter::Str(self.split(&['.', '[', ']'][..]))
    }
}

impl<'a> Into<SubKey<'a>> for usize {
    fn into(self) -> SubKey<'a> {
        SubKey::Int(self)
    }
}

impl<'a> Into<SubKeyIter<'a>> for usize {
    fn into(self) -> SubKeyIter<'a> {
        SubKeyIter::Int(Some(self))
    }
}

impl<'a> Into<SubKeyIter<'a>> for &'a String {
    fn into(self) -> SubKeyIter<'a> {
        self.as_str().into()
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

    macro_rules! should_iter {
        ($origin:literal: $($norm:literal),+) => {
            let mut che = CacheString::new();
            let mut key = che.new_key();
            key.push($origin);
            let mut iter = key.iter();
            $(
                let v: SubKey<'_> = $norm.into();
                assert_eq!(&v, iter.next().unwrap());
            )+
        };
    }

    #[test]
    fn key_iter_test() {
        should_iter!(
            "a.b[1][1].a[1]":
            "a", "b", 1, 1, "a", 1
        );
    }
}
