//! Words for what happens in a match.
//!
//! Built from the engine's action log and match events, so any engine
//! that emits those (tier 1 now, a spatial engine later) gets commentary
//! for free.

use crate::dto::{self, CommentaryKind};
use fs_match::{ActionKind, ActionRecord, GoalKind, MatchEvent, Outcome};

/// Names the commentary needs.
pub trait Names {
    /// A player's short name.
    fn player(&self, id: fs_core::ids::PlayerId) -> String;
    /// A side's club short name.
    fn team(&self, side: fs_match::Side) -> String;
}

/// A match clock string: `"12"`, `"45+2"`, `"90+4"`.
///
/// `first_half` says whether `elapsed` falls in the first half, and
/// `first_half_length` is how long the first half lasted (seconds).
pub fn clock(elapsed: f64, first_half: bool, first_half_length: f64) -> String {
    let (secs, cap) = if first_half {
        (elapsed, 45)
    } else {
        (elapsed - first_half_length + 45.0 * 60.0, 90)
    };
    let minute = (secs.max(0.0) / 60.0) as u32 + 1;
    if minute > cap {
        format!("{cap}+{}", minute - cap)
    } else {
        minute.to_string()
    }
}

fn line(
    t: f32,
    clock: String,
    kind: CommentaryKind,
    side: Option<fs_match::Side>,
    text: String,
) -> dto::Commentary {
    dto::Commentary {
        t,
        clock,
        kind,
        side: side.map(Into::into),
        text,
    }
}

/// A line for an action, if it is worth one. Goals, cards, substitutions
/// and injuries come from [`event`] instead.
pub fn action(a: &ActionRecord, clock: String, names: &impl Names) -> Option<dto::Commentary> {
    let who = || names.player(a.actor);
    let other = |id: Option<fs_core::ids::PlayerId>| id.map(|p| names.player(p));
    let side = Some(a.side);
    let big = a.xg >= 0.25;
    let (kind, text) = match (a.kind, a.outcome) {
        (_, Outcome::Goal) => return None,
        (ActionKind::Penalty, _) => return None,
        (ActionKind::Shot | ActionKind::Header | ActionKind::FreeKick, outcome) => {
            let what = match a.kind {
                ActionKind::Header => "header",
                ActionKind::FreeKick => "free kick",
                _ => "shot",
            };
            let text = match outcome {
                Outcome::Saved => match other(a.opponent) {
                    Some(k) => format!("{}'s {what} is saved by {k}.", who()),
                    None => format!("{}'s {what} is saved.", who()),
                },
                Outcome::Blocked => format!("{}'s {what} is blocked.", who()),
                _ if big => format!("{} should have scored! The {what} goes wide.", who()),
                _ => format!("{}'s {what} goes wide.", who()),
            };
            let kind = if outcome == Outcome::Saved {
                CommentaryKind::Save
            } else {
                CommentaryKind::Chance
            };
            (
                kind,
                if big && outcome != Outcome::Missed {
                    format!("Big chance! {text}")
                } else {
                    text
                },
            )
        }
        (ActionKind::Corner, Outcome::Cleared) => (
            CommentaryKind::SetPiece,
            format!("{} corner, cleared.", names.team(a.side)),
        ),
        (ActionKind::Corner, _) => (
            CommentaryKind::SetPiece,
            format!("Corner to {}. {} delivers.", names.team(a.side), who()),
        ),
        (ActionKind::Foul, _) => (
            CommentaryKind::Foul,
            match other(a.opponent) {
                Some(v) => format!("Foul by {} on {v}.", who()),
                None => format!("Foul by {}.", who()),
            },
        ),
        (_, Outcome::Offside) => (
            CommentaryKind::Play,
            match other(a.target) {
                Some(t) => format!("{t} is caught offside."),
                None => "Offside.".to_owned(),
            },
        ),
        (ActionKind::ThroughBall, Outcome::Complete) if a.to.band >= 4 => (
            CommentaryKind::Play,
            match other(a.target) {
                Some(t) => format!("{} slides {t} through!", who()),
                None => format!("{} plays a through ball.", who()),
            },
        ),
        (ActionKind::Dribble, Outcome::Complete) if a.from.band >= 4 => (
            CommentaryKind::Play,
            match other(a.opponent) {
                Some(o) => format!("{} skips past {o}.", who()),
                None => format!("{} goes on a run.", who()),
            },
        ),
        (ActionKind::Cross, Outcome::Complete) => (
            CommentaryKind::Play,
            match other(a.target) {
                Some(t) => format!("{} crosses for {t}.", who()),
                None => format!("{} crosses.", who()),
            },
        ),
        _ => return None,
    };
    Some(line(a.at, clock, kind, side, text))
}

fn goal_kind(k: GoalKind) -> &'static str {
    match k {
        GoalKind::OpenPlay => "",
        GoalKind::Header => " with a header",
        GoalKind::LongShot => " from distance",
        GoalKind::Penalty => " from the spot",
        GoalKind::FreeKick => " from a free kick",
        GoalKind::Corner => " from the corner",
    }
}

/// A line for a match event.
pub fn event(e: &MatchEvent, t: f32, clock: String, names: &impl Names) -> dto::Commentary {
    match *e {
        MatchEvent::Goal {
            side,
            scorer,
            assist,
            kind,
            ..
        } => {
            let assist = assist
                .map(|a| format!(" Assisted by {}.", names.player(a)))
                .unwrap_or_default();
            line(
                t,
                clock,
                CommentaryKind::Goal,
                Some(side),
                format!(
                    "GOAL! {} scores{} for {}.{assist}",
                    names.player(scorer),
                    goal_kind(kind),
                    names.team(side)
                ),
            )
        }
        MatchEvent::PenaltyMissed { side, taker, .. } => line(
            t,
            clock,
            CommentaryKind::Chance,
            Some(side),
            format!("{} misses the penalty!", names.player(taker)),
        ),
        MatchEvent::Card {
            side, player, red, ..
        } => line(
            t,
            clock,
            CommentaryKind::Card,
            Some(side),
            if red {
                format!("Red card! {} is sent off.", names.player(player))
            } else {
                format!("{} is booked.", names.player(player))
            },
        ),
        MatchEvent::Substitution { side, off, on, .. } => line(
            t,
            clock,
            CommentaryKind::Substitution,
            Some(side),
            format!(
                "{} change: {} replaces {}.",
                names.team(side),
                names.player(on),
                names.player(off)
            ),
        ),
        MatchEvent::Injury { side, player, .. } => line(
            t,
            clock,
            CommentaryKind::Injury,
            Some(side),
            format!("{} is hurt and needs treatment.", names.player(player)),
        ),
    }
}

/// A whistle: kick-off, half time, full time.
pub fn whistle(t: f32, clock: String, text: &str) -> dto::Commentary {
    line(t, clock, CommentaryKind::Whistle, None, text.to_owned())
}
