use std::{collections::HashSet, slice::Iter};

/// Config values.
#[derive(Debug)]
pub struct ConfigKey<'a> {
    node: Vec<KeyNode<'a>>,
}

impl<'a> ConfigKey<'a> {
    #[allow(dead_code)]
    pub(crate) fn new() -> Self {
        Self { node: vec![] }
    }
    #[allow(dead_code)]
    pub(crate) fn of(path: &'a str) -> Self {
        let mut key = Self::new();
        key.push(path);
        key
    }
}

#[allow(single_use_lifetimes)]
#[derive(Debug, PartialEq, Eq)]
pub enum SubKey<'a> {
    Str(&'a str),
    Int(usize),
}

#[derive(Debug)]
pub struct SubKeySeq<'a>(Vec<SubKey<'a>>);

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

    /// Add string key.
    pub(crate) fn insert_str(&mut self, key: &'a str) {
        if let Ok(i) = key.parse() {
            self.insert_int(i);
        } else {
            self.str_key.insert(key);
        }
    }

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
impl<'a> Into<SubKeySeq<'a>> for &'a str {
    fn into(self) -> SubKeySeq<'a> {
        SubKeySeq(
            self.split(&['.', '[', ']'][..])
                .filter(|a| !a.is_empty())
                .map(|f| match f.parse() {
                    Ok(v) => SubKey::Int(v),
                    _ => SubKey::Str(f),
                })
                .collect(),
        )
    }
}
impl<'a> Into<SubKey<'a>> for usize {
    fn into(self) -> SubKey<'a> {
        SubKey::Int(self)
    }
}

impl<'a> Into<SubKeySeq<'a>> for usize {
    fn into(self) -> SubKeySeq<'a> {
        SubKeySeq(vec![SubKey::Int(self)])
    }
}

#[derive(Debug)]
struct KeyNode<'a> {
    keys: Vec<SubKey<'a>>,
    key_long: String,
}

impl<'a> KeyNode<'a> {
    fn new(keys: Vec<SubKey<'a>>, parent: Option<&Self>) -> Self {
        let mut key_long = "".to_owned();
        if let Some(p) = parent {
            key_long.push_str(&p.key_long);
        }
        for k in keys.iter() {
            match k {
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
        Self { keys, key_long }
    }
}

/// Iterate config key with sub keys.
#[derive(Debug)]
pub struct KeyIter<'a, 'b> {
    ind: usize,
    iter: Vec<Iter<'b, SubKey<'a>>>,
}

impl<'a, 'b> Iterator for KeyIter<'a, 'b> {
    type Item = &'b SubKey<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        while self.ind < self.iter.len() {
            if let Some(v) = self.iter[self.ind].next() {
                return Some(v);
            }
            self.ind += 1;
        }
        None
    }
}

impl<'a> ConfigKey<'a> {
    /// Get normalized key as string.
    pub fn as_str(&self) -> &str {
        if let Some(v) = self.node.last() {
            return &v.key_long;
        }
        ""
    }

    /// Iterate config key as sub keys.
    pub fn iter(&self) -> KeyIter<'a, '_> {
        return KeyIter {
            ind: 0,
            iter: self.node.iter().map(|f| f.keys.iter()).collect(),
        };
    }

    #[allow(dead_code)]
    pub(crate) fn push<K: Into<SubKeySeq<'a>>>(&mut self, key: K) {
        self.node.push(KeyNode::new(key.into().0, self.node.last()));
    }

    #[allow(dead_code)]
    pub(crate) fn pop(&mut self) {
        self.node.pop();
    }
}

pub(crate) fn normalize_key(key: &str) -> String {
    let mut ck = ConfigKey::new();
    ck.push(key);
    ck.as_str().to_string()
}

#[cfg(test)]
mod test {
    use super::*;

    macro_rules! should_eq {
        ($origin:expr => $norm:expr) => {
            let mut key = ConfigKey::new();
            key.push($origin);
            assert_eq!(key.as_str(), $norm)
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
            let mut key = ConfigKey::new();
            let mut vec = vec![];
            $(
                key.push($origin);
                assert_eq!($norm, key.as_str());
                vec.push(key.as_str().to_owned());
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
            let mut key = ConfigKey::new();
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
