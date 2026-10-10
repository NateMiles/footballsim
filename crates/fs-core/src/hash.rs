//! Stable hashing of simulation state.
//!
//! A state hash is the 128-bit XXH3 of the value's `postcard` encoding.
//! Postcard is a fixed, platform-independent format, so equal states give
//! equal hashes on every target. This is what the determinism checks
//! compare.

use core::fmt;
use serde::Serialize;

/// A 128-bit hash of some simulation state.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct StateHash(pub u128);

impl fmt::Display for StateHash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:032x}", self.0)
    }
}

impl fmt::Debug for StateHash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "StateHash({self})")
    }
}

/// Hashes any serialisable value.
///
/// # Panics
/// Panics if the value fails to serialise, which only happens for types
/// with custom serialisers that return errors.
pub fn state_hash<T: Serialize + ?Sized>(value: &T) -> StateHash {
    let bytes = postcard::to_allocvec(value).expect("state must serialise");
    StateHash(xxhash_rust::xxh3::xxh3_128(&bytes))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn equal_values_equal_hashes() {
        assert_eq!(state_hash(&(1u32, "a")), state_hash(&(1u32, "a")));
        assert_ne!(state_hash(&(1u32, "a")), state_hash(&(2u32, "a")));
    }

    #[test]
    fn golden_hash() {
        // Pinned so a change in encoding or hash function is noticed.
        assert_eq!(
            state_hash(&(42u32, "footballsim")).to_string(),
            "d63a72aa4901c9bc38ab2ba3f357c261"
        );
    }
}
