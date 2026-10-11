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

/// A match clock string: `"12"`, `"45+2"`, `"90+4"`, `"105+1"`.
///
/// `half_starts` are the times (seconds since kickoff) at which each half
/// after the first began; extra time has two more.
pub fn clock(elapsed: f64, half_starts: &[f64]) -> String {
    const BASE: [f64; 4] = [0.0, 45.0, 90.0, 105.0];
    const CAP: [u32; 4] = [45, 90, 105, 120];
    let half = half_starts.iter().filter(|&&s| elapsed >= s).count().min(3);
    let start = if half == 0 {
        0.0
    } else {
        half_starts[half - 1]
    };
    let secs = elapsed - start + BASE[half] * 60.0;
    let minute = (secs.max(0.0) / 60.0) as u32 + 1;
    let cap = CAP[half];
    if minute > cap {
        format!("{cap}+{}", minute - cap)
    } else {
        minute.to_string()
    }
}

/// A line for a penalty in a shootout.
pub fn kick(
    t: f32,
    kick: &fs_match::shootout::Kick,
    score: [u8; 2],
    names: &dyn Names,
) -> dto::Commentary {
    let who = names.player(kick.taker);
    let text = if kick.scored {
        format!("{who} scores. {}-{}", score[0], score[1])
    } else {
        format!("{who} misses! {}-{}", score[0], score[1])
    };
    line(
        t,
        "Pens".into(),
        CommentaryKind::Shootout,
        Some(kick.side),
        text,
    )
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

/// A whistle: kick-off, full time.
pub fn whistle(t: f32, clock: String, text: &str) -> dto::Commentary {
    line(t, clock, CommentaryKind::Whistle, None, text.to_owned())
}

/// The whistle for a break between halves.
pub fn interval(t: f32, clock: String, text: &str) -> dto::Commentary {
    line(t, clock, CommentaryKind::Interval, None, text.to_owned())
}

#[cfg(test)]
mod tests {
    use super::clock;

    #[test]
    fn clocks_count_stoppage_and_extra_time() {
        assert_eq!(clock(0.0, &[]), "1");
        assert_eq!(clock(44.5 * 60.0, &[]), "45");
        assert_eq!(clock(46.5 * 60.0, &[]), "45+2");
        // The first half lasted 47 minutes.
        let starts = [47.0 * 60.0];
        assert_eq!(clock(47.0 * 60.0, &starts), "46");
        assert_eq!(clock(93.5 * 60.0, &starts), "90+2");
        // Extra time began after 95 minutes of play, its second half
        // after 111.
        let starts = [47.0 * 60.0, 95.0 * 60.0, 111.0 * 60.0];
        assert_eq!(clock(95.0 * 60.0, &starts), "91");
        assert_eq!(clock(110.5 * 60.0, &starts), "105+1");
        assert_eq!(clock(111.0 * 60.0, &starts), "106");
        assert_eq!(clock(126.5 * 60.0, &starts), "120+1");
    }
}
