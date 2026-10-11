//! A manager's chosen lineup, and how it becomes a team sheet.

use crate::ids::PlayerId;
use fs_match::selection::{BENCH_SIZE, suitability};
use fs_match::{PlayerSnapshot, Tactic, TeamSheet, pick_team};
use serde::{Deserialize, Serialize};

/// Eleven starters by formation slot, plus substitutes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Lineup {
    /// `starters[i]` plays formation slot `i` (slot 0 is the goalkeeper).
    pub starters: Vec<PlayerId>,
    pub bench: Vec<PlayerId>,
}

impl Lineup {
    /// Checks the shape: eleven distinct starters, at most nine
    /// substitutes, nobody listed twice.
    pub fn validate(&self) -> Result<(), String> {
        if self.starters.len() != 11 {
            return Err(format!("{} starters (need 11)", self.starters.len()));
        }
        if self.bench.len() > BENCH_SIZE {
            return Err(format!(
                "{} substitutes (at most {BENCH_SIZE})",
                self.bench.len()
            ));
        }
        let mut all: Vec<PlayerId> = self.starters.iter().chain(&self.bench).copied().collect();
        all.sort_unstable();
        if all.windows(2).any(|w| w[0] == w[1]) {
            return Err("a player is listed twice".into());
        }
        Ok(())
    }
}

/// Builds a team sheet from a chosen lineup and the available squad.
///
/// Chosen players who are no longer available (injured, suspended, gone)
/// are replaced by the most suitable available player for their slot, and
/// the bench is topped up, so a stale lineup never stops a match.
pub fn sheet_from_lineup(
    available: &[PlayerSnapshot],
    tactic: Tactic,
    lineup: &Lineup,
) -> TeamSheet {
    let find = |id: PlayerId| available.iter().find(|p| p.id == id);
    let slots = tactic.formation.slots();
    let mut used: Vec<PlayerId> = Vec::with_capacity(20);
    let mut starters: Vec<Option<PlayerSnapshot>> = lineup
        .starters
        .iter()
        .map(|&id| find(id).cloned())
        .collect();
    starters.resize(11, None);
    used.extend(starters.iter().flatten().map(|p| p.id));
    for (i, slot) in starters.iter_mut().enumerate() {
        if slot.is_none() {
            let position = slots[i].position;
            let best = available
                .iter()
                .filter(|p| !used.contains(&p.id) && !lineup.bench.contains(&p.id))
                .chain(available.iter().filter(|p| !used.contains(&p.id)))
                .max_by(|a, b| {
                    suitability(a, position)
                        .total_cmp(&suitability(b, position))
                        .then(b.id.cmp(&a.id))
                });
            if let Some(p) = best {
                used.push(p.id);
                *slot = Some(p.clone());
            }
        }
    }
    if starters.iter().any(Option::is_none) {
        // Not even eleven players: let automatic selection cope.
        return pick_team(available, tactic);
    }
    let starters: Vec<PlayerSnapshot> = starters.into_iter().flatten().collect();
    let mut bench: Vec<PlayerSnapshot> = lineup
        .bench
        .iter()
        .filter(|id| !used.contains(id))
        .filter_map(|&id| find(id).cloned())
        .collect();
    if bench.len() < BENCH_SIZE {
        // Top up with the automatic choice's substitutes.
        let auto = pick_team(available, tactic);
        for p in auto.starters.into_iter().chain(auto.bench) {
            if bench.len() == BENCH_SIZE {
                break;
            }
            if !used.contains(&p.id) && !bench.iter().any(|b| b.id == p.id) {
                bench.push(p);
            }
        }
    }
    TeamSheet {
        tactic,
        starters,
        bench,
        auto_subs: true,
        instructions: Default::default(),
    }
}
