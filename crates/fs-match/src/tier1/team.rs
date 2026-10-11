//! Teams and players as the tier 1 engine tracks them during a match.

use super::{BANDS, LANE_Y, LANES};
use crate::formation::Slot;
use crate::input::{PlayerSnapshot, Tactic, TeamSheet};
use crate::instruction::Shout;
use crate::result::{PlayerMatchStats, TeamStats};
use crate::roles::{Effect, Instructions, Marking};
use crate::skills::Skills;
use fs_core::attributes::Position;
use fs_core::math;

#[derive(Debug)]
pub(super) struct Player {
    pub(super) snap: PlayerSnapshot,
    pub(super) skills: Skills,
    pub(super) energy: f64,
    pub(super) slot: Option<usize>,
    /// Match minute the player came on (0 for starters).
    pub(super) on_minute: u8,
    /// Match minute the player went off, if they did.
    pub(super) off_minute: Option<u8>,
    pub(super) defensive: bool,
    pub(super) stats: PlayerMatchStats,
    /// Multiplier from a touchline shout (1.0 when none is active).
    pub(super) boost: f64,
}

impl Player {
    /// Multiplier from in-match fatigue.
    pub(super) fn fat(&self) -> f64 {
        (0.82 + 0.18 * self.energy) * self.boost
    }
}

pub(super) type Weights = [[[f64; 11]; LANES]; BANDS];

#[derive(Debug)]
pub(super) struct Team {
    pub(super) tactic: Tactic,
    /// How far from their usual position players join attacks in the
    /// final third (depth, as a share of the pitch).
    pub(super) final_third_spread: f64,
    pub(super) slots: [Slot; 11],
    pub(super) on: [Option<usize>; 11],
    pub(super) players: Vec<Player>,
    pub(super) bench: Vec<usize>,
    pub(super) subs_made: u8,
    pub(super) windows_used: u8,
    pub(super) attack_w: Box<Weights>,
    pub(super) defend_w: Box<Weights>,
    pub(super) stats: TeamStats,
    pub(super) in_play_secs: f64,
    pub(super) in_play_at_tick: f64,
    /// Whether the AI makes substitutions for tiredness (off when a human
    /// manages the side live).
    pub(super) auto_subs: bool,
    /// Minute of the last manual substitution window.
    pub(super) last_manual_sub: Option<u8>,
    pub(super) shout: Option<Shout>,
    pub(super) shout_until: f64,
    /// What each slot's role and duty change.
    pub(super) effects: [Effect; 11],
    /// Attacking presence in each zone relative to standard roles (1 when
    /// every role is standard).
    pub(super) presence: [[f64; LANES]; BANDS],
    /// Defensive coverage in each zone relative to standard roles.
    pub(super) coverage: [[f64; LANES]; BANDS],
    /// Set-piece takers, roles and opposition instructions.
    pub(super) instructions: Instructions,
}

impl Team {
    pub(super) fn new(sheet: &TeamSheet, final_third_spread: f64) -> Self {
        let tactic = sheet.tactic.clamped();
        let slots = tactic.formation.slots();
        let mut players = Vec::with_capacity(sheet.starters.len() + sheet.bench.len());
        let mut on = [None; 11];
        for (i, snap) in sheet.starters.iter().enumerate().take(11) {
            let position = slots[i].position;
            players.push(Player {
                snap: snap.clone(),
                skills: Skills::of(snap, position),
                energy: f64::from(snap.fitness) / 10_000.0,
                slot: Some(i),
                on_minute: 0,
                off_minute: None,
                defensive: is_defensive(position),
                stats: PlayerMatchStats::new(snap.id, true),
                boost: 1.0,
            });
            on[i] = Some(i);
        }
        let mut bench = Vec::with_capacity(sheet.bench.len());
        for snap in &sheet.bench {
            bench.push(players.len());
            players.push(Player {
                snap: snap.clone(),
                skills: Skills::default(),
                energy: f64::from(snap.fitness) / 10_000.0,
                slot: None,
                on_minute: 0,
                off_minute: None,
                defensive: false,
                stats: PlayerMatchStats::new(snap.id, false),
                boost: 1.0,
            });
        }
        let mut team = Self {
            tactic,
            final_third_spread,
            slots,
            on,
            players,
            bench,
            subs_made: 0,
            windows_used: 0,
            attack_w: Box::new([[[0.0; 11]; LANES]; BANDS]),
            defend_w: Box::new([[[0.0; 11]; LANES]; BANDS]),
            stats: TeamStats::default(),
            in_play_secs: 0.0,
            in_play_at_tick: 0.0,
            auto_subs: sheet.auto_subs,
            last_manual_sub: None,
            shout: None,
            shout_until: 0.0,
            effects: std::array::from_fn(|s| {
                Effect::of(slots[s].position, sheet.instructions.roles[s])
            }),
            presence: [[1.0; LANES]; BANDS],
            coverage: [[1.0; LANES]; BANDS],
            instructions: sheet.instructions.clone(),
        };
        team.rebuild_weights();
        team
    }

    /// Recomputes zone involvement after a lineup change.
    pub(super) fn rebuild_weights(&mut self) {
        let line = f64::from(self.tactic.defensive_line);
        let width = f64::from(self.tactic.width);
        let neutral = self.effects.iter().all(Effect::is_neutral);
        for band in 0..BANDS {
            let xb = (band as f64 + 0.5) / BANDS as f64;
            for (lane, &yl) in LANE_Y.iter().enumerate() {
                // Totals with standard roles, to measure what roles change.
                let (mut a0, mut d0) = (0.0, 0.0);
                for s in 0..11 {
                    if self.on[s].is_none() {
                        self.attack_w[band][lane][s] = 0.0;
                        self.defend_w[band][lane][s] = 0.0;
                        continue;
                    }
                    let slot = self.slots[s];
                    let (attack, defend) = if s == 0 {
                        let a = if band == 0 {
                            0.5
                        } else if band == 1 {
                            0.05
                        } else {
                            0.0
                        };
                        let d = if band == 0 { 0.25 } else { 0.0 };
                        (a, d)
                    } else {
                        // Wider teams spread players toward the touchlines.
                        let y = 0.5 + (slot.y - 0.5) * (1.0 + 0.08 * width);
                        // Midfielders arrive late in the box: involvement
                        // spreads further in the final third.
                        let spread = if band >= 4 {
                            self.final_third_spread
                        } else {
                            0.17
                        };
                        let a = gauss(slot.x - xb, spread) * gauss(y - yl, 0.26);
                        let dx = (slot.x * 0.7 + 0.04 * line).clamp(0.05, 0.9);
                        let d = gauss(dx - xb, 0.17) * gauss(slot.y - yl, 0.26);
                        a0 += a;
                        d0 += d;
                        let e = &self.effects[s];
                        if e.is_neutral() {
                            (a, d)
                        } else {
                            // Roles and duties move the player forward or
                            // back, inside, into the box, and change how
                            // much they defend.
                            let ya = if band >= 4 {
                                y + (0.5 - y) * e.inside
                            } else {
                                y
                            };
                            let mut a =
                                gauss(slot.x + e.forward - xb, spread) * gauss(ya - yl, 0.26);
                            if band == BANDS - 1 {
                                a *= e.in_box;
                            }
                            let dx =
                                ((slot.x + 0.5 * e.forward) * 0.7 + 0.04 * line).clamp(0.05, 0.9);
                            let d = gauss(dx - xb, 0.17) * gauss(slot.y - yl, 0.26) * e.defending;
                            (a, d)
                        }
                    };
                    self.attack_w[band][lane][s] = attack;
                    self.defend_w[band][lane][s] = defend;
                }
                if neutral {
                    continue;
                }
                let ratio = |w: &[f64; 11], base: f64| {
                    let outfield: f64 = w[1..].iter().sum();
                    if base > 0.0 {
                        (outfield / base).clamp(0.5, 2.0)
                    } else {
                        1.0
                    }
                };
                self.presence[band][lane] = ratio(&self.attack_w[band][lane], a0);
                self.coverage[band][lane] = ratio(&self.defend_w[band][lane], d0);
            }
        }
    }

    /// The set-piece taker the manager chose, if on the pitch, else the
    /// best by `skill`.
    pub(super) fn taker(
        &self,
        chosen: Option<fs_core::ids::PlayerId>,
        skill: impl Fn(&Skills) -> f64,
    ) -> usize {
        chosen
            .and_then(|id| self.on_pitch().find(|&i| self.players[i].snap.id == id))
            .unwrap_or_else(|| self.best_by(skill))
    }

    /// This team's instruction for an opposition player, if any.
    pub(super) fn marking(&self, id: fs_core::ids::PlayerId) -> Option<Marking> {
        self.instructions
            .marking
            .iter()
            .find(|m| m.0 == id)
            .map(|m| m.1)
    }

    /// How many of this team's instructions are of a kind.
    pub(super) fn markings(&self, kind: Marking) -> usize {
        self.instructions
            .marking
            .iter()
            .filter(|m| m.1 == kind)
            .count()
    }

    pub(super) fn player_in(&self, slot: usize) -> Option<usize> {
        self.on[slot]
    }

    pub(super) fn on_pitch(&self) -> impl Iterator<Item = usize> + '_ {
        self.on.iter().filter_map(|x| *x)
    }

    /// The player best at a skill among those on the pitch.
    pub(super) fn best_by(&self, f: impl Fn(&Skills) -> f64) -> usize {
        self.on_pitch()
            .max_by(|&a, &b| f(&self.players[a].skills).total_cmp(&f(&self.players[b].skills)))
            .expect("someone on the pitch")
    }
}

pub(super) fn gauss(d: f64, sigma: f64) -> f64 {
    math::exp(-(d * d) / (2.0 * sigma * sigma))
}

pub(super) fn is_defensive(p: Position) -> bool {
    matches!(
        p,
        Position::Goalkeeper
            | Position::DefenderCentre
            | Position::DefenderLeft
            | Position::DefenderRight
            | Position::WingBackLeft
            | Position::WingBackRight
    )
}
