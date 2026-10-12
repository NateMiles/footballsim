//! Post-match analysis from the action log: shots, possession over the
//! match, where each side had the ball, and who passed to whom.
//!
//! Built from [`ActionRecord`]s, so any engine that emits them (tier 1
//! now, a spatial tier 0 later) gets the same analysis.

use crate::log::{ActionKind, ActionRecord, Outcome, Zone};
use crate::result::Side;
use fs_core::ids::PlayerId;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

const BANDS: usize = 6;
const LANES: usize = 3;
/// Seconds in each possession spell.
const SPELL: f32 = 15.0 * 60.0;

/// One shot.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ShotRecord {
    pub minute: u8,
    pub side: Side,
    pub player: PlayerId,
    /// Shot, header, penalty or free kick.
    pub kind: ActionKind,
    /// Where it was taken from, in the shooting side's frame.
    pub zone: Zone,
    pub xg: f32,
    /// Goal, saved, missed or blocked.
    pub outcome: Outcome,
}

/// Where a player was involved on average, in their side's frame.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct PlayerPosition {
    pub player: PlayerId,
    /// 0 (own goal line) to 5 (the opponent's box).
    pub band: f32,
    /// 0 (left) to 2 (right).
    pub lane: f32,
    /// Actions and passes received.
    pub involvements: u16,
}

/// Completed passes from one player to another.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PassLink {
    pub from: PlayerId,
    pub to: PlayerId,
    pub count: u16,
}

/// One side's match on the ball.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct SideAnalysis {
    /// Actions started in each zone, by band then lane, in the side's
    /// frame.
    pub touches: [[u16; LANES]; BANDS],
    pub positions: Vec<PlayerPosition>,
    /// Most frequent first.
    pub passes: Vec<PassLink>,
}

/// A match analysed.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct MatchAnalysis {
    /// In match order.
    pub shots: Vec<ShotRecord>,
    /// Home then away.
    pub sides: [SideAnalysis; 2],
    /// The home side's share of possession in each 15-minute spell.
    pub possession: Vec<f32>,
}

fn is_shot(kind: ActionKind) -> bool {
    matches!(
        kind,
        ActionKind::Shot | ActionKind::Header | ActionKind::Penalty | ActionKind::FreeKick
    )
}

fn is_pass(kind: ActionKind) -> bool {
    matches!(
        kind,
        ActionKind::ShortPass | ActionKind::LongPass | ActionKind::ThroughBall | ActionKind::Cross
    )
}

/// Analyses a match from its action log.
pub fn analyse(log: &[ActionRecord]) -> MatchAnalysis {
    let mut a = MatchAnalysis::default();
    // Per side: player -> (band sum, lane sum, count).
    let mut places: [BTreeMap<PlayerId, (f32, f32, u16)>; 2] = Default::default();
    let mut links: [BTreeMap<(PlayerId, PlayerId), u16>; 2] = Default::default();
    let mut spells: Vec<[f32; 2]> = Vec::new();
    for r in log {
        let s = r.side.index();
        if r.kind != ActionKind::Foul {
            let z = r.from;
            a.sides[s].touches[usize::from(z.band).min(BANDS - 1)]
                [usize::from(z.lane).min(LANES - 1)] += 1;
            let e = places[s].entry(r.actor).or_default();
            *e = (e.0 + f32::from(z.band), e.1 + f32::from(z.lane), e.2 + 1);
        }
        if is_shot(r.kind)
            && matches!(
                r.outcome,
                Outcome::Goal | Outcome::Saved | Outcome::Missed | Outcome::Blocked
            )
        {
            a.shots.push(ShotRecord {
                minute: (r.at / 60.0) as u8,
                side: r.side,
                player: r.actor,
                kind: r.kind,
                zone: r.from,
                xg: r.xg,
                outcome: r.outcome,
            });
        }
        if is_pass(r.kind)
            && r.outcome == Outcome::Complete
            && let Some(to) = r.target
        {
            *links[s].entry((r.actor, to)).or_default() += 1;
            let e = places[s].entry(to).or_default();
            *e = (
                e.0 + f32::from(r.to.band),
                e.1 + f32::from(r.to.lane),
                e.2 + 1,
            );
        }
        let spell = (r.at / SPELL) as usize;
        if spells.len() <= spell {
            spells.resize(spell + 1, [0.0; 2]);
        }
        spells[spell][s] += r.duration;
    }
    for s in 0..2 {
        a.sides[s].positions = places[s]
            .iter()
            .map(|(&player, &(b, l, n))| PlayerPosition {
                player,
                band: b / f32::from(n.max(1)),
                lane: l / f32::from(n.max(1)),
                involvements: n,
            })
            .collect();
        let mut v: Vec<PassLink> = links[s]
            .iter()
            .map(|(&(from, to), &count)| PassLink { from, to, count })
            .collect();
        v.sort_by(|x, y| {
            y.count
                .cmp(&x.count)
                .then((x.from, x.to).cmp(&(y.from, y.to)))
        });
        a.sides[s].passes = v;
    }
    a.possession = spells
        .iter()
        .map(|p| {
            let total = p[0] + p[1];
            if total > 0.0 { p[0] / total } else { 0.5 }
        })
        .collect();
    a
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::input;
    use crate::tier1;

    #[test]
    fn analysis_agrees_with_the_result() {
        let (r, log) = tier1::play_recorded(&input(600, 550, 3));
        let a = analyse(&log);
        for s in 0..2 {
            let side = Side::from_index(s);
            let shots: Vec<_> = a.shots.iter().filter(|x| x.side == side).collect();
            assert_eq!(shots.len(), usize::from(r.stats[s].shots), "side {s}");
            let goals = shots.iter().filter(|x| x.outcome == Outcome::Goal).count();
            assert_eq!(goals, usize::from(r.goals[s]));
            let passes: u32 = a.sides[s].passes.iter().map(|l| u32::from(l.count)).sum();
            assert!(passes > 100 && passes <= u32::from(r.stats[s].passes_completed));
        }
        assert!(a.possession.len() >= 6);
        let home: f32 = a.possession.iter().sum::<f32>() / a.possession.len() as f32;
        assert!((home - r.stats[0].possession).abs() < 0.1);
    }
}
