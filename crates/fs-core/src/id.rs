//! Dense, typed entity IDs and the arrays they index.
//!
//! Every entity is identified by a `u32` index into a per-type array. IDs of
//! different entity types cannot be mixed up because each has its own type.

use core::fmt;
use core::marker::PhantomData;
use serde::{Deserialize, Serialize};

/// Implemented by the marker types that tag an [`Id`].
pub trait IdKind: 'static {
    /// Short prefix used when printing, such as `"player"`.
    const NAME: &'static str;
}

/// A typed index into an [`IdVec`].
#[derive(Serialize, Deserialize)]
#[serde(transparent)]
pub struct Id<K> {
    raw: u32,
    #[serde(skip)]
    _kind: PhantomData<fn() -> K>,
}

impl<K> Id<K> {
    /// Wraps a raw index.
    #[inline]
    pub const fn new(raw: u32) -> Self {
        Self {
            raw,
            _kind: PhantomData,
        }
    }

    /// The raw index.
    #[inline]
    pub const fn raw(self) -> u32 {
        self.raw
    }

    /// The raw index as a `usize`, for slice indexing.
    #[inline]
    pub const fn index(self) -> usize {
        self.raw as usize
    }
}

// Manual impls so `K` does not need to implement these traits itself.
impl<K> Clone for Id<K> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<K> Copy for Id<K> {}
impl<K> PartialEq for Id<K> {
    fn eq(&self, other: &Self) -> bool {
        self.raw == other.raw
    }
}
impl<K> Eq for Id<K> {}
impl<K> PartialOrd for Id<K> {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
impl<K> Ord for Id<K> {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        self.raw.cmp(&other.raw)
    }
}
impl<K> core::hash::Hash for Id<K> {
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        self.raw.hash(state);
    }
}
impl<K: IdKind> fmt::Debug for Id<K> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}#{}", K::NAME, self.raw)
    }
}
impl<K: IdKind> fmt::Display for Id<K> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self, f)
    }
}

/// Declares an ID marker type and a type alias for its [`Id`].
///
/// ```
/// fs_core::define_id!(PlayerKind, PlayerId, "player");
/// let id = PlayerId::new(3);
/// assert_eq!(format!("{id}"), "player#3");
/// ```
#[macro_export]
macro_rules! define_id {
    ($kind:ident, $alias:ident, $name:literal) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        #[doc = concat!("Marker type for [`", stringify!($alias), "`].")]
        pub enum $kind {}
        impl $crate::id::IdKind for $kind {
            const NAME: &'static str = $name;
        }
        #[doc = concat!("ID of a ", $name, ".")]
        pub type $alias = $crate::id::Id<$kind>;
    };
}

/// A vector indexed by a typed [`Id`]. Entities are never removed, so IDs
/// stay valid for the life of a world.
#[derive(Serialize, Deserialize, Clone, PartialEq, Eq)]
#[serde(transparent)]
pub struct IdVec<K, T> {
    items: Vec<T>,
    #[serde(skip)]
    _kind: PhantomData<fn() -> K>,
}

impl<K, T> Default for IdVec<K, T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<K, T: fmt::Debug> fmt::Debug for IdVec<K, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(&self.items).finish()
    }
}

impl<K, T> IdVec<K, T> {
    /// An empty vector.
    pub const fn new() -> Self {
        Self {
            items: Vec::new(),
            _kind: PhantomData,
        }
    }

    /// An empty vector with room for `n` items.
    pub fn with_capacity(n: usize) -> Self {
        Self {
            items: Vec::with_capacity(n),
            _kind: PhantomData,
        }
    }

    /// Appends an item and returns its new ID.
    ///
    /// # Panics
    /// Panics if more than `u32::MAX` items are stored.
    pub fn push(&mut self, item: T) -> Id<K> {
        let raw = u32::try_from(self.items.len()).expect("IdVec overflow");
        self.items.push(item);
        Id::new(raw)
    }

    /// Number of items.
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Whether there are no items.
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// The item with this ID, if it exists.
    pub fn get(&self, id: Id<K>) -> Option<&T> {
        self.items.get(id.index())
    }

    /// The item with this ID, mutably, if it exists.
    pub fn get_mut(&mut self, id: Id<K>) -> Option<&mut T> {
        self.items.get_mut(id.index())
    }

    /// Iterates over IDs in ascending order.
    pub fn ids(&self) -> impl ExactSizeIterator<Item = Id<K>> + use<K, T> {
        (0..self.items.len() as u32).map(Id::new)
    }

    /// Iterates over `(id, item)` pairs in ascending ID order.
    pub fn iter(&self) -> impl ExactSizeIterator<Item = (Id<K>, &T)> {
        self.items
            .iter()
            .enumerate()
            .map(|(i, t)| (Id::new(i as u32), t))
    }

    /// The items as a slice, in ID order.
    pub fn as_slice(&self) -> &[T] {
        &self.items
    }

    /// The items as a mutable slice, in ID order.
    pub fn as_mut_slice(&mut self) -> &mut [T] {
        &mut self.items
    }
}

impl<K, T> core::ops::Index<Id<K>> for IdVec<K, T> {
    type Output = T;
    fn index(&self, id: Id<K>) -> &T {
        &self.items[id.index()]
    }
}

impl<K, T> core::ops::IndexMut<Id<K>> for IdVec<K, T> {
    fn index_mut(&mut self, id: Id<K>) -> &mut T {
        &mut self.items[id.index()]
    }
}

impl<K, T> FromIterator<T> for IdVec<K, T> {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        Self {
            items: iter.into_iter().collect(),
            _kind: PhantomData,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::define_id!(TestKind, TestId, "test");

    #[test]
    fn push_and_index() {
        let mut v: IdVec<TestKind, &str> = IdVec::new();
        let a = v.push("a");
        let b = v.push("b");
        assert_eq!(a, TestId::new(0));
        assert_eq!(v[b], "b");
        assert_eq!(v.ids().collect::<Vec<_>>(), vec![a, b]);
        assert_eq!(format!("{b:?}"), "test#1");
    }

    #[test]
    fn serde_is_transparent() {
        let id = TestId::new(42);
        let bytes = postcard::to_allocvec(&id).unwrap();
        assert_eq!(bytes, postcard::to_allocvec(&42u32).unwrap());
        let back: TestId = postcard::from_bytes(&bytes).unwrap();
        assert_eq!(back, id);
    }
}
