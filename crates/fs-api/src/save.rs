//! Saved games, in the `fs-save` container.
//!
//! The session's own state (which manager is the viewer, and their inbox)
//! travels in the container's host section. A match being watched is not
//! saved: the match day stays pending in the world, so loading returns to
//! kickoff. The base64 wrapping is for carrying saves inside JSON.

use crate::dto;
use crate::inbox::Inbox;
use crate::protocol::{ApiError, ErrorCode};
use crate::query::refs::date;
use crate::session::Career;
use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;
use fs_world::Control;
use fs_world::{Game, ManagerId};
use serde::{Deserialize, Serialize};

/// The session's part of a save.
#[derive(Serialize, Deserialize)]
struct Host {
    manager: u32,
    inbox: Inbox,
}

fn summary(c: &Career) -> fs_save::Summary {
    let world = c.game.world();
    let m = &world.managers()[c.manager];
    fs_save::Summary {
        date: date(world.date()),
        manager: format!(
            "{} {}",
            world.names().get(m.first_name),
            world.names().get(m.last_name)
        ),
        club: m.club.map_or_else(
            || "Unemployed".to_owned(),
            |c| world.clubs()[c].name.clone(),
        ),
    }
}

pub(crate) fn save(c: &Career) -> dto::SaveGame {
    let summary = summary(c);
    let host = serde_json::to_vec(&Host {
        manager: c.manager.raw(),
        inbox: c.inbox.clone(),
    })
    .expect("host state serialises");
    let bytes = fs_save::write(fs_save::Contents {
        game: &c.game,
        summary: &summary,
        origin: None,
        host: &host,
    });
    dto::SaveGame {
        data: STANDARD.encode(bytes),
        summary: dto::SaveSummary {
            date: summary.date,
            manager: summary.manager,
            club: summary.club,
        },
    }
}

pub(crate) fn load(data: &str) -> Result<(Game, ManagerId, Inbox), ApiError> {
    let bad = |m: String| ApiError::new(ErrorCode::BadSave, m);
    let bytes = STANDARD
        .decode(data.trim())
        .map_err(|e| bad(e.to_string()))?;
    let loaded = fs_save::read(&bytes).map_err(|e| bad(e.to_string()))?;
    let host: Host = serde_json::from_slice(&loaded.host).map_err(|e| bad(e.to_string()))?;
    let manager = ManagerId::new(host.manager);
    let world = loaded.game.world();
    if world
        .managers()
        .get(manager)
        .is_none_or(|m| m.control != Control::Human)
    {
        return Err(bad("the save's manager is missing".into()));
    }
    Ok((loaded.game, manager, host.inbox))
}
