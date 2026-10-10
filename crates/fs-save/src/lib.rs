//! Saved games.
//!
//! A save is a small container:
//!
//! ```text
//! b"FSSV"  container version (u16 LE)  header length (u32 LE)  header (JSON)
//! section bytes, one after another, as the header lists them
//! ```
//!
//! The JSON header carries what a load screen shows (date, manager,
//! club), the versions needed to read the rest, and for each section its
//! name, codec, sizes and an XXH3 checksum. Reading the header alone is
//! cheap, so save lists never decode worlds.
//!
//! Sections:
//! - `world`: the world snapshot (`World::to_snapshot`), deflated.
//! - `log`: the command log since the game began (or was first loaded),
//!   postcard, deflated. Kept for replays and bug reports.
//! - `origin` (optional): the world the log starts from. With it, a save
//!   can be replayed command by command to check determinism.
//! - `host`: opaque bytes belonging to the program that saved (the UI
//!   session's inbox, for example).
//!
//! The world snapshot carries its own format version
//! (`fs_world::snapshot::FORMAT_VERSION`). Loading an older one runs it
//! through [`MIGRATIONS`] first. Snapshots are postcard, which has no
//! field names, so a migration decodes the old layout with a frozen copy
//! of the old types and re-encodes the new one. None exist yet: worlds
//! before the first release are not migrated.

use fs_world::snapshot::{FORMAT_VERSION, MAGIC as WORLD_MAGIC};
use fs_world::{CommandRecord, Game, SnapshotError, World, replay};
use serde::{Deserialize, Serialize};

/// File signature.
pub const MAGIC: [u8; 4] = *b"FSSV";
/// Container layout version. Bump when the container itself changes.
pub const CONTAINER_VERSION: u16 = 1;

/// Why a save could not be read.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SaveError {
    #[error("not a footballsim save")]
    BadMagic,
    #[error("save container version {0} is not supported")]
    UnsupportedContainer(u16),
    #[error("the save's header is damaged: {0}")]
    BadHeader(String),
    #[error("the save has no {0} section")]
    MissingSection(&'static str),
    #[error("the {0} section is damaged")]
    Corrupt(String),
    #[error("world format {found} cannot be migrated to {wanted}")]
    NoMigration { found: u16, wanted: u16 },
    #[error("the world could not be read: {0}")]
    World(#[from] SnapshotError),
    #[error("replaying the log gave a different world")]
    ReplayMismatch,
}

/// What a save list shows without loading the world.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Summary {
    /// The in-game date, `YYYY-MM-DD`.
    pub date: String,
    pub manager: String,
    pub club: String,
}

/// One section of the container.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Section {
    pub name: String,
    /// `"deflate"` or `"none"`.
    pub codec: String,
    /// Bytes after decompression.
    pub raw_len: u64,
    /// Bytes stored.
    pub len: u64,
    /// XXH3-64 of the stored bytes, hex.
    pub checksum: String,
}

/// The container header.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Header {
    /// The program version that saved.
    pub game_version: String,
    /// `fs_world::snapshot::FORMAT_VERSION` of the world section.
    pub world_format: u16,
    pub summary: Summary,
    pub sections: Vec<Section>,
}

/// A loaded save.
#[derive(Debug, Clone)]
pub struct Loaded {
    pub header: Header,
    /// The game, with its command log.
    pub game: Game,
    /// The world the log starts from, if the save kept it.
    pub origin: Option<World>,
    pub host: Vec<u8>,
}

/// A migration of the world snapshot from one format to the next.
#[derive(Debug, Clone, Copy)]
pub struct Migration {
    pub from: u16,
    /// Takes a whole snapshot (magic and version included) in format
    /// `from` and returns one in format `from + 1`.
    pub run: fn(&[u8]) -> Result<Vec<u8>, SaveError>,
}

/// Known migrations, oldest first.
pub const MIGRATIONS: &[Migration] = &[];

/// What to put in a save.
#[derive(Debug, Clone, Copy)]
pub struct Contents<'a> {
    pub game: &'a Game,
    pub summary: &'a Summary,
    /// The world the log starts from, to make the save replayable.
    pub origin: Option<&'a World>,
    pub host: &'a [u8],
}

fn checksum(bytes: &[u8]) -> String {
    format!("{:016x}", xxhash_rust::xxh3::xxh3_64(bytes))
}

/// Writes a save.
///
/// # Panics
/// Panics only if the command log fails to serialise, which cannot happen
/// for its types.
pub fn write(contents: Contents<'_>) -> Vec<u8> {
    let mut sections: Vec<(&'static str, Vec<u8>, bool)> = vec![
        ("world", contents.game.world().to_snapshot(), true),
        (
            "log",
            postcard::to_allocvec(contents.game.log()).expect("log serialises"),
            true,
        ),
    ];
    if let Some(origin) = contents.origin {
        sections.push(("origin", origin.to_snapshot(), true));
    }
    sections.push(("host", contents.host.to_vec(), false));

    let mut table = Vec::new();
    let mut body = Vec::new();
    for (name, raw, compress) in sections {
        let stored = if compress {
            // Level 1: most of the gain for a fraction of the time.
            miniz_oxide::deflate::compress_to_vec(&raw, 1)
        } else {
            raw.clone()
        };
        table.push(Section {
            name: name.into(),
            codec: if compress { "deflate" } else { "none" }.into(),
            raw_len: raw.len() as u64,
            len: stored.len() as u64,
            checksum: checksum(&stored),
        });
        body.extend_from_slice(&stored);
    }
    let header = Header {
        game_version: env!("CARGO_PKG_VERSION").into(),
        world_format: FORMAT_VERSION,
        summary: contents.summary.clone(),
        sections: table,
    };
    let header = serde_json::to_vec(&header).expect("header serialises");
    let mut out = Vec::with_capacity(10 + header.len() + body.len());
    out.extend_from_slice(&MAGIC);
    out.extend_from_slice(&CONTAINER_VERSION.to_le_bytes());
    out.extend_from_slice(&(header.len() as u32).to_le_bytes());
    out.extend_from_slice(&header);
    out.extend_from_slice(&body);
    out
}

/// Reads the header and returns it with the offset of the first section.
fn split(bytes: &[u8]) -> Result<(Header, usize), SaveError> {
    let rest = bytes.strip_prefix(&MAGIC).ok_or(SaveError::BadMagic)?;
    let (version, rest) = rest.split_first_chunk::<2>().ok_or(SaveError::BadMagic)?;
    let version = u16::from_le_bytes(*version);
    if version != CONTAINER_VERSION {
        return Err(SaveError::UnsupportedContainer(version));
    }
    let (len, rest) = rest
        .split_first_chunk::<4>()
        .ok_or_else(|| SaveError::BadHeader("truncated".into()))?;
    let len = u32::from_le_bytes(*len) as usize;
    let json = rest
        .get(..len)
        .ok_or_else(|| SaveError::BadHeader("truncated".into()))?;
    let header: Header =
        serde_json::from_slice(json).map_err(|e| SaveError::BadHeader(e.to_string()))?;
    Ok((header, MAGIC.len() + 2 + 4 + len))
}

/// Reads only the header: cheap, for listing saves.
pub fn read_header(bytes: &[u8]) -> Result<Header, SaveError> {
    split(bytes).map(|(h, _)| h)
}

/// Sections' names and decoded bytes, in order.
type Decoded = Vec<(String, Vec<u8>)>;

/// The decoded bytes of every section, by name.
fn sections(bytes: &[u8]) -> Result<(Header, Decoded), SaveError> {
    let (header, mut at) = split(bytes)?;
    let mut out = Vec::with_capacity(header.sections.len());
    for s in &header.sections {
        let end = at + s.len as usize;
        let stored = bytes
            .get(at..end)
            .ok_or_else(|| SaveError::Corrupt(s.name.clone()))?;
        if checksum(stored) != s.checksum {
            return Err(SaveError::Corrupt(s.name.clone()));
        }
        let raw = match s.codec.as_str() {
            "deflate" => {
                miniz_oxide::inflate::decompress_to_vec_with_limit(stored, s.raw_len as usize)
                    .map_err(|_| SaveError::Corrupt(s.name.clone()))?
            }
            "none" => stored.to_vec(),
            _ => return Err(SaveError::Corrupt(s.name.clone())),
        };
        if raw.len() as u64 != s.raw_len {
            return Err(SaveError::Corrupt(s.name.clone()));
        }
        out.push((s.name.clone(), raw));
        at = end;
    }
    Ok((header, out))
}

/// The format version inside a world snapshot.
fn snapshot_version(snapshot: &[u8]) -> Option<u16> {
    let rest = snapshot.strip_prefix(&WORLD_MAGIC)?;
    rest.first_chunk::<2>().map(|v| u16::from_le_bytes(*v))
}

/// Brings an old world snapshot up to the current format.
pub fn migrate(mut snapshot: Vec<u8>) -> Result<Vec<u8>, SaveError> {
    loop {
        let found = snapshot_version(&snapshot).ok_or(SaveError::World(SnapshotError::BadMagic))?;
        if found == FORMAT_VERSION {
            return Ok(snapshot);
        }
        let step = MIGRATIONS
            .iter()
            .find(|m| m.from == found)
            .ok_or(SaveError::NoMigration {
                found,
                wanted: FORMAT_VERSION,
            })?;
        snapshot = (step.run)(&snapshot)?;
    }
}

fn world_from(snapshot: Vec<u8>) -> Result<World, SaveError> {
    Ok(World::from_snapshot(&migrate(snapshot)?)?)
}

/// Reads a whole save.
pub fn read(bytes: &[u8]) -> Result<Loaded, SaveError> {
    let (header, mut secs) = sections(bytes)?;
    let mut take = |name: &'static str| {
        secs.iter()
            .position(|(n, _)| n == name)
            .map(|i| secs.swap_remove(i).1)
    };
    let world = world_from(take("world").ok_or(SaveError::MissingSection("world"))?)?;
    let log: Vec<CommandRecord> = match take("log") {
        Some(b) => postcard::from_bytes(&b).map_err(|_| SaveError::Corrupt("log".into()))?,
        None => Vec::new(),
    };
    let origin = take("origin").map(world_from).transpose()?;
    let host = take("host").unwrap_or_default();
    Ok(Loaded {
        header,
        game: Game::resume(world, log),
        origin,
        host,
    })
}

/// Replays a save's log from its origin and checks the result matches
/// the saved world: a determinism check on a real game.
pub fn verify(loaded: &Loaded) -> Result<(), SaveError> {
    let origin = loaded
        .origin
        .clone()
        .ok_or(SaveError::MissingSection("origin"))?;
    let replayed = replay(origin, loaded.game.log()).map_err(|_| SaveError::ReplayMismatch)?;
    if replayed.world().state_hash() == loaded.game.world().state_hash() {
        Ok(())
    } else {
        Err(SaveError::ReplayMismatch)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fs_gen::{GenConfig, Size, generate};
    use fs_world::{Actor, Command};

    fn summary() -> Summary {
        Summary {
            date: "2026-07-01".into(),
            manager: "A Manager".into(),
            club: "A Club".into(),
        }
    }

    fn played() -> (World, Game) {
        let origin = generate(&GenConfig::preset(Size::Tiny, 51));
        let mut game = Game::new(origin.clone());
        game.submit(Actor::System, Command::Advance { days: 45 })
            .unwrap();
        (origin, game)
    }

    #[test]
    fn a_save_round_trips_and_replays() {
        let (origin, game) = played();
        let bytes = write(Contents {
            game: &game,
            summary: &summary(),
            origin: Some(&origin),
            host: b"inbox",
        });
        assert_eq!(read_header(&bytes).unwrap().summary, summary());
        let loaded = read(&bytes).unwrap();
        assert_eq!(loaded.game.world().state_hash(), game.world().state_hash());
        assert_eq!(loaded.game.log(), game.log());
        assert_eq!(loaded.host, b"inbox");
        verify(&loaded).unwrap();
        // Compression earns its keep.
        let raw = game.world().to_snapshot().len() + origin.to_snapshot().len();
        assert!(bytes.len() < raw * 3 / 4, "{} vs {raw}", bytes.len());
    }

    #[test]
    fn damage_is_detected() {
        let (_, game) = played();
        let mut bytes = write(Contents {
            game: &game,
            summary: &summary(),
            origin: None,
            host: &[],
        });
        assert_eq!(read(b"nope").unwrap_err(), SaveError::BadMagic);
        let n = bytes.len();
        bytes[n - 100] ^= 0xff;
        assert!(matches!(read(&bytes), Err(SaveError::Corrupt(_))));
    }

    #[test]
    fn unknown_world_formats_are_refused() {
        let (_, game) = played();
        let mut snap = game.world().to_snapshot();
        snap[4] = 1;
        snap[5] = 0;
        assert_eq!(
            migrate(snap).unwrap_err(),
            SaveError::NoMigration {
                found: 1,
                wanted: FORMAT_VERSION
            }
        );
    }
}
