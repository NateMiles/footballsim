//! Save snapshots of a [`World`].
//!
//! Layout: 4-byte magic `FSIM`, little-endian `u16` format version, then
//! the `postcard` encoding of the world. Compression and migrations for
//! older versions arrive with the save system in M4.

use crate::world::World;

/// File signature.
pub const MAGIC: [u8; 4] = *b"FSIM";
/// Current snapshot format version. Bump on any change to serialised types.
pub const FORMAT_VERSION: u16 = 2;

/// Why a snapshot could not be loaded.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SnapshotError {
    #[error("not a footballsim snapshot")]
    BadMagic,
    #[error("snapshot format version {0} is not supported (this build reads {FORMAT_VERSION})")]
    UnsupportedVersion(u16),
    #[error("snapshot is damaged: {0}")]
    Decode(String),
    #[error("snapshot is inconsistent: {0}")]
    Corrupt(String),
}

impl World {
    /// Encodes the world as a snapshot.
    ///
    /// # Panics
    /// Panics only if serialisation fails, which cannot happen for the
    /// types in a world.
    pub fn to_snapshot(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(1024);
        out.extend_from_slice(&MAGIC);
        out.extend_from_slice(&FORMAT_VERSION.to_le_bytes());
        postcard::to_extend(self, out).expect("world must serialise")
    }

    /// Decodes and validates a snapshot.
    pub fn from_snapshot(bytes: &[u8]) -> Result<World, SnapshotError> {
        let rest = bytes.strip_prefix(&MAGIC).ok_or(SnapshotError::BadMagic)?;
        let (version, body) = rest
            .split_first_chunk::<2>()
            .ok_or(SnapshotError::BadMagic)?;
        let version = u16::from_le_bytes(*version);
        if version != FORMAT_VERSION {
            return Err(SnapshotError::UnsupportedVersion(version));
        }
        let world: World =
            postcard::from_bytes(body).map_err(|e| SnapshotError::Decode(e.to_string()))?;
        world.check_invariants().map_err(SnapshotError::Corrupt)?;
        Ok(world)
    }
}
