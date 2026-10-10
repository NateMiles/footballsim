//! Saved games.
//!
//! A save holds the world snapshot (see `fs_world::snapshot`), the
//! viewer and their inbox. A match being watched is not saved: the match
//! day stays pending in the world, so loading returns to kickoff.
//!
//! The command log is not kept yet; full replays from saves arrive with
//! the save system in M4.

use crate::dto;
use crate::inbox::Inbox;
use crate::protocol::{ApiError, ErrorCode};
use crate::query::refs::date;
use crate::session::Career;
use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;
use fs_world::{Game, ManagerId, World};
use serde::{Deserialize, Serialize};

/// Save layout version. Bump on any change to [`SaveFile`].
pub const SAVE_VERSION: u16 = 1;

#[derive(Serialize, Deserialize)]
struct SaveFile {
    version: u16,
    manager: u32,
    /// `World::to_snapshot`.
    world: Vec<u8>,
    /// The inbox as JSON.
    inbox: String,
}

pub(crate) fn save(c: &Career) -> dto::SaveGame {
    let world = c.game.world();
    let file = SaveFile {
        version: SAVE_VERSION,
        manager: c.manager.raw(),
        world: world.to_snapshot(),
        inbox: serde_json::to_string(&c.inbox).expect("inbox serialises"),
    };
    let bytes = postcard::to_allocvec(&file).expect("save serialises");
    let m = &world.managers()[c.manager];
    dto::SaveGame {
        data: STANDARD.encode(bytes),
        summary: dto::SaveSummary {
            date: date(world.date()),
            manager: format!(
                "{} {}",
                world.names().get(m.first_name),
                world.names().get(m.last_name)
            ),
            club: m
                .club
                .map(|c| world.clubs()[c].name.clone())
                .unwrap_or_default(),
        },
    }
}

pub(crate) fn load(data: &str) -> Result<(Game, ManagerId, Inbox), ApiError> {
    let bad = |m: String| ApiError::new(ErrorCode::BadSave, m);
    let bytes = STANDARD
        .decode(data.trim())
        .map_err(|e| bad(e.to_string()))?;
    let file: SaveFile = postcard::from_bytes(&bytes).map_err(|e| bad(e.to_string()))?;
    if file.version != SAVE_VERSION {
        return Err(bad(format!(
            "save version {} is not supported",
            file.version
        )));
    }
    let world = World::from_snapshot(&file.world).map_err(|e| bad(e.to_string()))?;
    let manager = ManagerId::new(file.manager);
    if world.human_club(manager).is_none() {
        return Err(bad("the save's manager has no club".into()));
    }
    let inbox: Inbox = serde_json::from_str(&file.inbox).map_err(|e| bad(e.to_string()))?;
    Ok((Game::new(world), manager, inbox))
}
