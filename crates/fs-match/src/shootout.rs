//! Penalty shootouts, shared by both engines.

use crate::input::{PlayerSnapshot, TeamSheet};
use crate::result::Side;
use crate::skills::Skills;
use fs_core::Rng;
use fs_core::attributes::Position;
use fs_core::ids::PlayerId;
use serde::{Deserialize, Serialize};

/// Kicks each side takes before sudden death.
const ROUNDS: usize = 5;
/// Scoring chance for an average taker against an average keeper.
const BASE: f64 = 0.76;
/// Keeper skill the base chance assumes (as in tier 1).
const AVERAGE_KEEPER: f64 = 0.82;

/// One kick.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Kick {
    pub side: Side,
    pub taker: PlayerId,
    pub scored: bool,
}

/// A shootout, kick by kick.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Shootout {
    pub kicks: Vec<Kick>,
    /// Penalties scored, home then away.
    pub score: [u8; 2],
}

impl Shootout {
    /// The side that won.
    pub fn winner(&self) -> Side {
        if self.score[0] > self.score[1] {
            Side::Home
        } else {
            Side::Away
        }
    }
}

/// A player able to take a kick.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Taker {
    pub id: PlayerId,
    /// Penalty skill, 0-1.
    pub penalty: f64,
}

/// One side at the end of play: who is on the pitch, keeper first.
#[derive(Debug, Clone, PartialEq)]
pub struct Lineup {
    pub takers: Vec<Taker>,
    /// Keeper skill, 0-1.
    pub keeper: f64,
}

impl Lineup {
    /// Builds a lineup from snapshots on the pitch, the first being in
    /// goal. Fatigue multiplies every skill.
    pub fn new(on_pitch: &[(&PlayerSnapshot, f64)]) -> Self {
        let keeper = on_pitch.first().map_or(0.3, |(p, fatigue)| {
            Skills::of(p, Position::Goalkeeper).keeper * fatigue
        });
        let takers = on_pitch
            .iter()
            .map(|(p, fatigue)| Taker {
                id: p.id,
                penalty: Skills::of(p, p.positions.best()).penalty * fatigue,
            })
            .collect();
        Self { takers, keeper }
    }

    /// The starters of a sheet, all fresh (for tests and tier 2).
    pub fn from_sheet(sheet: &TeamSheet) -> Self {
        let on: Vec<(&PlayerSnapshot, f64)> = sheet.starters.iter().map(|p| (p, 1.0)).collect();
        Self::new(&on)
    }
}

/// Chance a kick goes in.
fn chance(taker: f64, keeper: f64, pressure: bool) -> f64 {
    let p = BASE * (0.85 + 0.25 * taker) * (1.0 + 1.2 * (AVERAGE_KEEPER - keeper));
    let p = if pressure { p * (0.9 + 0.1 * taker) } else { p };
    p.clamp(0.45, 0.95)
}

/// Plays a shootout. The best takers go first; once everyone has taken
/// one, the order starts again. `first` kicks first.
pub fn play(rng: &mut Rng, sides: [&Lineup; 2], first: Side) -> Shootout {
    let order: [Vec<Taker>; 2] = sides.map(|l| {
        let mut t = l.takers.clone();
        // Keepers last, then by skill.
        t[1..].sort_by(|a, b| b.penalty.total_cmp(&a.penalty).then(a.id.cmp(&b.id)));
        t.rotate_left(1);
        t
    });
    let mut out = Shootout::default();
    let mut taken = [0usize; 2];
    let sides_in_order = [first, first.other()];
    loop {
        for side in sides_in_order {
            let s = side.index();
            let o = 1 - s;
            let list = &order[s];
            if list.is_empty() {
                continue;
            }
            let taker = list[taken[s] % list.len()];
            // A kick that must be scored to stay alive.
            let must_score =
                (taken[s] >= ROUNDS - 1 && out.score[s] < out.score[o]) || taken[s] >= ROUNDS;
            let scored = rng.chance(chance(taker.penalty, sides[o].keeper, must_score));
            taken[s] += 1;
            if scored {
                out.score[s] += 1;
            }
            out.kicks.push(Kick {
                side,
                taker: taker.id,
                scored,
            });
            if decided(out.score, taken) {
                return out;
            }
        }
    }
}

/// Whether the shootout is over.
fn decided(score: [u8; 2], taken: [usize; 2]) -> bool {
    let left = |s: usize| ROUNDS.saturating_sub(taken[s]) as u8;
    if taken[0] < ROUNDS || taken[1] < ROUNDS {
        // Best of five: over once one side cannot catch up.
        score[0] + left(0) < score[1] || score[1] + left(1) < score[0]
    } else {
        taken[0] == taken[1] && score[0] != score[1]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::sheet;

    /// Eleven takers of one standard and a typical keeper.
    fn side(penalty: f64, first_id: u32) -> Lineup {
        Lineup {
            takers: (0..11)
                .map(|i| Taker {
                    id: PlayerId::new(first_id + i),
                    penalty,
                })
                .collect(),
            keeper: AVERAGE_KEEPER,
        }
    }

    #[test]
    fn shootouts_end_with_a_winner() {
        let home = side(0.6, 0);
        let away = side(0.6, 100);
        let mut home_wins = 0;
        let mut kicks = 0;
        for k in 0..2_000u64 {
            let mut rng = Rng::from_u64(k);
            let s = play(&mut rng, [&home, &away], Side::Home);
            assert_ne!(s.score[0], s.score[1]);
            let n = |side| s.kicks.iter().filter(|x| x.side == side).count();
            let (h, a) = (n(Side::Home), n(Side::Away));
            assert!(h == a || h == a + 1, "{h} v {a}");
            let scored = |side| {
                s.kicks
                    .iter()
                    .filter(|x| x.side == side && x.scored)
                    .count() as u8
            };
            assert_eq!([scored(Side::Home), scored(Side::Away)], s.score);
            home_wins += usize::from(s.winner() == Side::Home);
            kicks += s.kicks.len();
        }
        // Even sides: close to a coin toss, with about ten kicks.
        assert!((800..1200).contains(&home_wins), "{home_wins}");
        let mean = kicks as f64 / 2_000.0;
        assert!((8.5..12.0).contains(&mean), "{mean}");
    }

    #[test]
    fn better_takers_win_more() {
        let good = side(0.85, 0);
        let poor = side(0.35, 100);
        let wins = (0..2_000u64)
            .filter(|&k| {
                let mut rng = Rng::from_u64(k);
                play(&mut rng, [&good, &poor], Side::Away).winner() == Side::Home
            })
            .count();
        assert!(wins > 1_150, "{wins}");
    }

    #[test]
    fn lineups_come_from_sheets() {
        let l = Lineup::from_sheet(&sheet(600, 0));
        assert_eq!(l.takers.len(), 11);
        let mut rng = Rng::from_u64(9);
        let s = play(
            &mut rng,
            [&l, &Lineup::from_sheet(&sheet(600, 100))],
            Side::Home,
        );
        assert_eq!(s.kicks[0].side, Side::Home);
        // The keeper is not among the first five takers.
        assert!(s.kicks.iter().take(9).all(|k| k.taker != PlayerId::new(0)));
    }

    #[test]
    fn best_of_five_stops_early() {
        assert!(decided([3, 0], [3, 3]));
        assert!(!decided([3, 1], [3, 3]));
        assert!(decided([4, 2], [4, 4]));
        assert!(!decided([5, 5], [5, 5]));
        assert!(decided([6, 5], [6, 6]));
        assert!(!decided([6, 5], [6, 5]));
    }
}
