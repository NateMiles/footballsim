//! Interned name strings.
//!
//! Hundreds of thousands of people share a few thousand first and last
//! names, so people store a 4-byte [`NameRef`] instead of a `String`.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Index of a string in a [`NamePool`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct NameRef(pub u32);

/// A deduplicated table of name strings.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct NamePool {
    strings: Vec<String>,
    #[serde(skip)]
    lookup: BTreeMap<String, NameRef>,
}

// Equality ignores the lookup cache, which is rebuilt lazily after loading.
impl PartialEq for NamePool {
    fn eq(&self, other: &Self) -> bool {
        self.strings == other.strings
    }
}
impl Eq for NamePool {}

impl NamePool {
    /// Returns the reference for `name`, adding it if it is new.
    pub fn intern(&mut self, name: &str) -> NameRef {
        if self.lookup.is_empty() && !self.strings.is_empty() {
            self.rebuild_lookup();
        }
        if let Some(r) = self.lookup.get(name) {
            return *r;
        }
        let r = NameRef(u32::try_from(self.strings.len()).expect("name pool overflow"));
        self.strings.push(name.to_owned());
        self.lookup.insert(name.to_owned(), r);
        r
    }

    /// The string for a reference.
    ///
    /// # Panics
    /// Panics if the reference came from a different pool.
    pub fn get(&self, r: NameRef) -> &str {
        &self.strings[r.0 as usize]
    }

    /// Number of distinct strings.
    pub fn len(&self) -> usize {
        self.strings.len()
    }

    /// Whether the pool is empty.
    pub fn is_empty(&self) -> bool {
        self.strings.is_empty()
    }

    fn rebuild_lookup(&mut self) {
        self.lookup = self
            .strings
            .iter()
            .enumerate()
            .map(|(i, s)| (s.clone(), NameRef(i as u32)))
            .collect();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn interning_dedups() {
        let mut p = NamePool::default();
        let a = p.intern("Silva");
        let b = p.intern("Moreau");
        assert_eq!(p.intern("Silva"), a);
        assert_eq!(p.get(b), "Moreau");
        assert_eq!(p.len(), 2);
    }

    #[test]
    fn lookup_survives_serde() {
        let mut p = NamePool::default();
        let a = p.intern("Silva");
        let bytes = postcard::to_allocvec(&p).unwrap();
        let mut back: NamePool = postcard::from_bytes(&bytes).unwrap();
        assert_eq!(back.intern("Silva"), a);
        assert_eq!(back.len(), 1);
    }
}
