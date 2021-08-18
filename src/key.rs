use std::{
    collections::{hash_map::DefaultHasher, HashSet},
    hash::{Hash, Hasher},
    slice::Iter,
};
///Config Values
pub type ConfigKey<'a> = HashKey<'a, DefaultHasher>;
// pub type ConfigKey<'a> = DefaultKey<'a>;

/// Config values.
#[derive(Debug)]
#[allow(unreachable_pub)]
pub struct DefaultKey<'a> {
    node: Vec<KeyNode<'a>>,
}

impl<'a> DefaultKey<'a> {
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

impl Default for DefaultKey<'_> {
    fn default() -> Self {
        Self::new()
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
#[allow(unreachable_pub)]
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

impl<'a> DefaultKey<'a> {
    /// Get normalized key as string.
    pub(crate) fn as_str(&self) -> &str {
        if let Some(v) = self.node.last() {
            return &v.key_long;
        }
        ""
    }

    #[allow(dead_code)]
    pub(crate) fn to_string(&self) -> String {
        self.as_str().to_string()
    }

    #[allow(dead_code)]
    /// Iterate config key as sub keys.
    pub(crate) fn iter(&self) -> KeyIter<'a, '_> {
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
    let mut ck = DefaultKey::new();
    ck.push(key);
    ck.as_str().to_string()
}

#[cfg(test)]
mod test {
    use super::*;

    macro_rules! should_eq {
        ($origin:expr => $norm:expr) => {
            let mut key = DefaultKey::new();
            key.push($origin);
            assert_eq!(key.as_str(), $norm);

            let mut key = HashKey::default();
            key.push($origin);
            assert_eq!(&key.to_string(), $norm);

            let mut kez = HashKey::default();
            kez.push($norm);
            assert_eq!(true, &key == &kez);
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
            let mut key = DefaultKey::new();
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

            let mut key = HashKey::default();
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
            let mut key = DefaultKey::new();
            key.push($origin);
            let mut iter = key.iter();
            $(
                let v: SubKey<'_> = $norm.into();
                assert_eq!(&v, iter.next().unwrap());
            )+

            let mut key = HashKey::default();
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

impl Hash for SubKey<'_> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self {
            SubKey::Str(i) => (*i).hash(state),
            SubKey::Int(i) => i.hash(state),
        }
    }
}

#[allow(dead_code)]
#[derive(Debug)]
pub struct HashKey<'a, H: Hasher> {
    hasher: H,
    current: String,
    sub: Vec<usize>,
    keys: Vec<(H, SubKey<'a>, String)>,
}

impl<H: Hasher> Hash for HashKey<'_, H> {
    fn hash<X: Hasher>(&self, state: &mut X) {
        self.hasher.finish().hash(state);
    }
}

impl<H: Hasher> PartialEq for HashKey<'_, H> {
    fn eq(&self, other: &Self) -> bool {
        if self.hasher.finish() != other.hasher.finish() {
            return false;
        }
        self.current == other.current
    }
}

impl<H: Hasher> Eq for HashKey<'_, H> {}

impl Default for HashKey<'_, DefaultHasher> {
    fn default() -> Self {
        Self::new(DefaultHasher::new())
    }
}

impl<'a, H: Hasher + Clone> HashKey<'a, H> {
    pub(crate) fn new(hasher: H) -> Self {
        Self {
            hasher,
            current: "".to_string(),
            sub: vec![],
            keys: vec![],
        }
    }

    #[allow(dead_code)]
    pub(crate) fn push<K: Into<SubKeySeq<'a>>>(&mut self, key: K) {
        let v: SubKeySeq<'a> = key.into();
        let mut size = 0;
        for sub in v.0 {
            size += 1;
            let curr = self.current.clone();
            match &sub {
                SubKey::Str(i) => {
                    if !self.current.is_empty() {
                        self.current.push('.');
                    }
                    self.current.push_str(i);
                }
                SubKey::Int(i) => {
                    self.current.push('[');
                    self.current.push_str(&i.to_string());
                    self.current.push(']');
                }
            }
            let hash = self.hasher.clone();
            sub.hash(&mut self.hasher);
            self.keys.push((hash, sub, curr));
        }
        self.sub.push(size);
    }

    #[allow(dead_code)]
    pub(crate) fn pop(&mut self) {
        if let Some(s) = self.sub.pop() {
            if s > 0 {
                for (h, _, c) in self.keys.drain(self.keys.len() - s..) {
                    self.hasher = h;
                    self.current = c;
                    return;
                }
            }
        }
    }

    #[allow(dead_code)]
    pub(crate) fn iter(&self) -> HashKeyIter<'_, H> {
        HashKeyIter(self.keys.iter())
    }

    /// As string
    #[allow(dead_code)]
    pub(crate) fn as_str(&self) -> &str {
        &self.current
    }

    /// To String.
    #[allow(dead_code)]
    pub(crate) fn to_string(&self) -> String {
        self.as_str().to_string()
    }
}

pub(crate) struct HashKeyIter<'a, H>(Iter<'a, (H, SubKey<'a>, String)>);

impl<'a, H> Iterator for HashKeyIter<'a, H> {
    type Item = &'a SubKey<'a>;
    fn next(&mut self) -> Option<Self::Item> {
        self.0.next().map(|i| &i.1)
    }
}
