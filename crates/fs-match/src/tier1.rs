//! Tier 1: event-level match simulation.
//!
//! The pitch is a grid of six bands (own goal to opponent goal, from the
//! attacking team's point of view) by three lanes (left, centre, right).
//! The team in possession repeatedly picks an action for the ball carrier:
//! short pass, long ball, through ball, cross, dribble or shot. Each action
//! is a contest between the carrier's skill and a nearby defender's,
//! resolved with a logistic curve. Who is "nearby" comes from each team's
//! formation: players near a zone are more often involved there.
//!
//! The engine also handles fouls, cards, penalties, set pieces, injuries,
//! in-match fatigue and substitutions. All tunable numbers live in
//! [`Params`] so the calibration tools can fit them.

use crate::formation::Slot;
use crate::input::{MatchInput, PlayerSnapshot, Tactic, TeamSheet};
use crate::result::{
    GoalKind, MatchEvent, MatchResult, PlayerMatchStats, Side, TeamStats, rate_players,
};
use crate::skills::Skills;
use fs_core::attributes::Position;
use fs_core::{Rng, math};

const BANDS: usize = 6;
const LANES: usize = 3;
const HALF_SECS: f64 = 45.0 * 60.0;
const LANE_Y: [f64; LANES] = [0.15, 0.5, 0.85];
const MAX_SUBS: u8 = 5;
const MAX_SUB_WINDOWS: u8 = 3;

/// Tunable engine constants. [`Params::default`] holds the calibrated
/// values.
#[derive(Debug, Clone, PartialEq)]
pub struct Params {
    /// Logit change per unit of skill difference in a contest.
    pub skill_slope: f64,
    /// Logit change per unit of skill difference for short passes. Higher
    /// values let technically better teams dominate the ball.
    pub pass_slope: f64,
    /// Logit bonus for the home side's actions.
    pub home_logit: f64,
    /// Base logit of a short pass succeeding, by band.
    pub short_base: [f64; BANDS],
    pub long_base: f64,
    pub through_base: f64,
    pub cross_base: f64,
    pub dribble_base: f64,
    /// Probability per action of a foul away from the ball contest.
    pub foul_rate: f64,
    /// Probability a failed dribble is a foul.
    pub dribble_foul: f64,
    /// Probability a foul earns a yellow card, before temperament.
    pub card_rate: f64,
    pub straight_red: f64,
    /// Base xG by band (5 = penalty area) for central and wide lanes.
    pub xg_central: [f64; BANDS],
    pub xg_wide: [f64; BANDS],
    pub xg_big_chance: f64,
    pub xg_header: f64,
    pub xg_corner_header: f64,
    pub xg_penalty: f64,
    pub xg_free_kick: f64,
    /// Probability a shot is on target, before shooter skill.
    pub on_target_base: f64,
    /// Seconds a short pass takes.
    pub pass_secs: f64,
    /// Probability per action of an injury to an involved player.
    pub injury_rate: f64,
    /// Energy lost per minute on the pitch by an average player.
    pub energy_drain: f64,
    /// Outfielders below this energy are substituted when possible.
    pub sub_energy: f64,
    /// Share of short passes played forward a band.
    pub forward_rate: f64,
    /// Logit penalty for a forward pass (sideways passes get +0.2,
    /// backward +0.6).
    pub forward_risk: f64,
    /// Weight of a shot from the box, before player and tactic modifiers.
    pub box_shot_weight: f64,
    /// Keeper skill (after fatigue) that xG values assume.
    pub average_keeper: f64,
    /// How much each directness step changes the long-ball rate.
    pub direct_effect: f64,
    /// Logit pressure per pressing step in the opponent's half.
    pub press_effect: f64,
    /// Logit bonus near goal per step of the defending side's mentality.
    pub exposure_effect: f64,
    /// Change in the forward-pass share per directness step.
    pub direct_forward: f64,
    /// Depth spread of attacking involvement in the final third; larger
    /// values spread shots and goals beyond the strikers.
    pub final_third_spread: f64,
    /// Extra energy drain from chasing: a side that spends the whole
    /// minute without the ball drains `1 + chase_fatigue / 2` times as fast.
    pub chase_fatigue: f64,
}

impl Default for Params {
    /// Values fitted with `fs-lab`'s calibration report
    /// (docs/CALIBRATION.md).
    fn default() -> Self {
        Self {
            skill_slope: 3.6,
            pass_slope: 5.5,
            home_logit: 0.1,
            short_base: [3.8, 3.4, 1.4, 0.8, 0.5, 0.1],
            long_base: -0.6,
            through_base: -1.7,
            cross_base: -2.3,
            dribble_base: -0.1,
            foul_rate: 0.015,
            dribble_foul: 0.22,
            card_rate: 0.17,
            straight_red: 0.0015,
            xg_central: [0.0, 0.0, 0.0, 0.02, 0.045, 0.10],
            xg_wide: [0.0, 0.0, 0.0, 0.01, 0.025, 0.05],
            xg_big_chance: 0.3,
            xg_header: 0.07,
            xg_corner_header: 0.07,
            xg_penalty: 0.76,
            xg_free_kick: 0.055,
            on_target_base: 0.12,
            pass_secs: 3.3,
            injury_rate: 0.0012,
            energy_drain: 0.0045,
            sub_energy: 0.74,
            forward_rate: 0.25,
            forward_risk: 1.2,
            box_shot_weight: 0.16,
            average_keeper: 0.82,
            direct_effect: 0.6,
            press_effect: 0.15,
            exposure_effect: 0.12,
            chase_fatigue: 1.0,
            final_third_spread: 0.3,
            direct_forward: 0.0,
        }
    }
}

/// Plays a match with the calibrated parameters.
pub fn play(input: &MatchInput) -> MatchResult {
    play_with(input, &Params::default())
}

/// Plays a match with custom parameters.
pub fn play_with(input: &MatchInput, params: &Params) -> MatchResult {
    debug_assert_eq!(input.validate(), Ok(()));
    Engine::new(input, params).run()
}

#[derive(Debug)]
struct Player<'a> {
    snap: &'a PlayerSnapshot,
    skills: Skills,
    energy: f64,
    slot: Option<usize>,
    /// Match minute the player came on (0 for starters).
    on_minute: u8,
    /// Match minute the player went off, if they did.
    off_minute: Option<u8>,
    defensive: bool,
    stats: PlayerMatchStats,
}

impl Player<'_> {
    /// Multiplier from in-match fatigue.
    fn fat(&self) -> f64 {
        0.82 + 0.18 * self.energy
    }
}

type Weights = [[[f64; 11]; LANES]; BANDS];

#[derive(Debug)]
struct Team<'a> {
    tactic: Tactic,
    /// How far from their usual position players join attacks in the
    /// final third (depth, as a share of the pitch).
    final_third_spread: f64,
    slots: [Slot; 11],
    on: [Option<usize>; 11],
    players: Vec<Player<'a>>,
    bench: Vec<usize>,
    subs_made: u8,
    windows_used: u8,
    attack_w: Box<Weights>,
    defend_w: Box<Weights>,
    stats: TeamStats,
    in_play_secs: f64,
    in_play_at_tick: f64,
}

impl<'a> Team<'a> {
    fn new(sheet: &'a TeamSheet, final_third_spread: f64) -> Self {
        let tactic = sheet.tactic.clamped();
        let slots = tactic.formation.slots();
        let mut players = Vec::with_capacity(sheet.starters.len() + sheet.bench.len());
        let mut on = [None; 11];
        for (i, snap) in sheet.starters.iter().enumerate().take(11) {
            let position = slots[i].position;
            players.push(Player {
                snap,
                skills: Skills::of(snap, position),
                energy: f64::from(snap.fitness) / 10_000.0,
                slot: Some(i),
                on_minute: 0,
                off_minute: None,
                defensive: is_defensive(position),
                stats: PlayerMatchStats::new(snap.id, true),
            });
            on[i] = Some(i);
        }
        let mut bench = Vec::with_capacity(sheet.bench.len());
        for snap in &sheet.bench {
            bench.push(players.len());
            players.push(Player {
                snap,
                skills: Skills::default(),
                energy: f64::from(snap.fitness) / 10_000.0,
                slot: None,
                on_minute: 0,
                off_minute: None,
                defensive: false,
                stats: PlayerMatchStats::new(snap.id, false),
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
        };
        team.rebuild_weights();
        team
    }

    /// Recomputes zone involvement after a lineup change.
    fn rebuild_weights(&mut self) {
        let line = f64::from(self.tactic.defensive_line);
        let width = f64::from(self.tactic.width);
        for band in 0..BANDS {
            let xb = (band as f64 + 0.5) / BANDS as f64;
            for (lane, &yl) in LANE_Y.iter().enumerate() {
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
                        (a, d)
                    };
                    self.attack_w[band][lane][s] = attack;
                    self.defend_w[band][lane][s] = defend;
                }
            }
        }
    }

    fn player_in(&self, slot: usize) -> Option<usize> {
        self.on[slot]
    }

    fn on_pitch(&self) -> impl Iterator<Item = usize> + '_ {
        self.on.iter().filter_map(|x| *x)
    }

    /// The player best at a skill among those on the pitch.
    fn best_by(&self, f: impl Fn(&Skills) -> f64) -> usize {
        self.on_pitch()
            .max_by(|&a, &b| f(&self.players[a].skills).total_cmp(&f(&self.players[b].skills)))
            .expect("someone on the pitch")
    }
}

fn gauss(d: f64, sigma: f64) -> f64 {
    math::exp(-(d * d) / (2.0 * sigma * sigma))
}

fn is_defensive(p: Position) -> bool {
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

#[derive(Debug, Clone, Copy)]
struct Ball {
    side: usize,
    band: usize,
    lane: usize,
    /// Slot of the player on the ball.
    carrier: usize,
    /// Player index of the last successful passer in this possession.
    last_passer: Option<usize>,
    /// The carrier just beat a defender in the box.
    beat_man: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Action {
    Short,
    Long,
    Through,
    Cross,
    Dribble,
    Shot,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ShotKind {
    Open,
    Long,
    Big,
    Header,
    CornerHeader,
    Penalty,
    FreeKick,
}

struct Engine<'a> {
    p: &'a Params,
    rng: Rng,
    teams: [Team<'a>; 2],
    ball: Ball,
    t: f64,
    half: u8,
    neutral: bool,
    goals: [u8; 2],
    events: Vec<MatchEvent>,
}

impl<'a> Engine<'a> {
    fn new(input: &'a MatchInput, p: &'a Params) -> Self {
        let mut e = Self {
            p,
            rng: input.rng(),
            teams: [
                Team::new(&input.home, p.final_third_spread),
                Team::new(&input.away, p.final_third_spread),
            ],
            ball: Ball {
                side: 0,
                band: 2,
                lane: 1,
                carrier: 1,
                last_passer: None,
                beat_man: false,
            },
            t: 0.0,
            half: 0,
            neutral: input.neutral,
            goals: [0, 0],
            events: Vec::with_capacity(16),
        };
        e.kickoff(0);
        e
    }

    fn minute(&self) -> u8 {
        let m = (self.t / 60.0) as u32 + 1;
        let cap = if self.half == 0 { 45 } else { 90 };
        m.min(cap) as u8
    }

    fn run(mut self) -> MatchResult {
        let stoppage1 = 60.0 + f64::from(self.rng.below(150));
        let stoppage2 = 120.0 + f64::from(self.rng.below(300));
        let mut end = HALF_SECS + stoppage1;
        let mut next_minute = 60.0;
        loop {
            if self.t >= end {
                if self.half == 0 {
                    self.half = 1;
                    self.t = HALF_SECS;
                    next_minute = HALF_SECS + 60.0;
                    end = 2.0 * HALF_SECS + stoppage2;
                    self.half_time_recovery();
                    self.kickoff(1);
                    continue;
                }
                break;
            }
            while self.t >= next_minute {
                self.minute_tick(next_minute);
                next_minute += 60.0;
            }
            let dt = self.step();
            self.t += dt;
        }
        self.finish()
    }

    fn kickoff(&mut self, side: usize) {
        let w = self.teams[side].attack_w[2][1];
        let carrier = self.rng.weighted_index(&w).unwrap_or(1);
        self.ball = Ball {
            side,
            band: 2,
            lane: 1,
            carrier,
            last_passer: None,
            beat_man: false,
        };
    }

    fn half_time_recovery(&mut self) {
        for team in &mut self.teams {
            for p in &mut team.players {
                p.energy = (p.energy + 0.06).min(1.0);
            }
        }
    }

    fn minute_tick(&mut self, at: f64) {
        let minute = (at / 60.0).round() as u32;
        // Time on the ball since the last tick: chasing the ball is tiring.
        let played = [
            self.teams[0].in_play_secs - self.teams[0].in_play_at_tick,
            self.teams[1].in_play_secs - self.teams[1].in_play_at_tick,
        ];
        let total = (played[0] + played[1]).max(1.0);
        for side in 0..2 {
            let chasing = played[1 - side] / total;
            let team = &mut self.teams[side];
            team.in_play_at_tick = team.in_play_secs;
            let chase = 1.0 + self.p.chase_fatigue * (chasing - 0.5);
            let press = f64::from(team.tactic.pressing);
            let tempo = f64::from(team.tactic.tempo);
            for s in 0..11 {
                if let Some(i) = team.on[s] {
                    let pl = &mut team.players[i];
                    let mut drain = self.p.energy_drain
                        * (1.3 - 0.6 * pl.skills.stamina)
                        * (1.0 + 0.08 * press + 0.05 * tempo)
                        * chase;
                    if s == 0 {
                        drain *= 0.3;
                    }
                    pl.energy = (pl.energy - drain).max(0.0);
                }
            }
            match minute {
                60 => self.ai_substitutions(side, 2, self.p.sub_energy),
                70 => self.ai_substitutions(side, 2, self.p.sub_energy + 0.04),
                80 => self.ai_substitutions(side, 2, self.p.sub_energy + 0.08),
                _ => {}
            }
        }
    }

    /// Replaces outfielders below `threshold` energy, tiredest first, up to
    /// `max` in this window.
    fn ai_substitutions(&mut self, side: usize, max: u8, threshold: f64) {
        let team = &self.teams[side];
        if team.windows_used >= MAX_SUB_WINDOWS
            || team.subs_made >= MAX_SUBS
            || team.bench.is_empty()
        {
            return;
        }
        let mut tired: Vec<(usize, f64)> = (1..11)
            .filter_map(|s| team.on[s].map(|i| (s, team.players[i].energy)))
            .filter(|&(_, e)| e < threshold)
            .collect();
        tired.sort_by(|a, b| a.1.total_cmp(&b.1).then(a.0.cmp(&b.0)));
        let mut made = 0;
        for (slot, _) in tired {
            if made == max || self.teams[side].subs_made >= MAX_SUBS {
                break;
            }
            if self.substitute(side, slot) {
                made += 1;
            }
        }
        if made > 0 {
            self.teams[side].windows_used += 1;
        }
    }

    /// Brings on the best bench player for `slot`. Returns false if nobody
    /// suitable is left.
    fn substitute(&mut self, side: usize, slot: usize) -> bool {
        let minute = self.minute();
        let team = &mut self.teams[side];
        let position = team.slots[slot].position;
        let Some(k) = (0..team.bench.len()).max_by(|&a, &b| {
            let sa = Skills::of(team.players[team.bench[a]].snap, position).overall;
            let sb = Skills::of(team.players[team.bench[b]].snap, position).overall;
            sa.total_cmp(&sb).then(b.cmp(&a))
        }) else {
            return false;
        };
        let incoming = team.bench.remove(k);
        let outgoing = team.on[slot].expect("slot occupied");
        team.players[outgoing].slot = None;
        team.players[outgoing].off_minute = Some(minute);
        let pl = &mut team.players[incoming];
        pl.skills = Skills::of(pl.snap, position);
        pl.slot = Some(slot);
        pl.on_minute = minute;
        pl.stats.minutes = 1; // marks the player as having appeared
        pl.defensive = is_defensive(position);
        team.on[slot] = Some(incoming);
        team.subs_made += 1;
        let (off, on) = (
            team.players[outgoing].snap.id,
            team.players[incoming].snap.id,
        );
        self.events.push(MatchEvent::Substitution {
            minute,
            side: Side::from_index(side),
            off,
            on,
        });
        team.rebuild_weights();
        true
    }

    fn pick(
        &mut self,
        side: usize,
        band: usize,
        lane: usize,
        attack: bool,
        exclude: Option<usize>,
    ) -> usize {
        let team = &self.teams[side];
        let mut w = if attack {
            team.attack_w[band][lane]
        } else {
            team.defend_w[band][lane]
        };
        if let Some(x) = exclude {
            w[x] = 0.0;
        }
        match self.rng.weighted_index(&w) {
            Some(s) => s,
            // Nobody near the zone: fall back to anyone on the pitch.
            None => {
                let on: Vec<usize> = (0..11)
                    .filter(|&s| team.on[s].is_some() && Some(s) != exclude)
                    .collect();
                *self.rng.choose(&on).unwrap_or(&0)
            }
        }
    }

    fn home_logit(&self, side: usize) -> f64 {
        if self.neutral {
            0.0
        } else if side == 0 {
            self.p.home_logit
        } else {
            -self.p.home_logit
        }
    }

    /// Mentality after reacting to the score: trailing sides push on as
    /// time runs out; comfortable leaders sit back.
    fn mentality(&self, side: usize) -> f64 {
        let base = f64::from(self.teams[side].tactic.mentality);
        let diff = i32::from(self.goals[side]) - i32::from(self.goals[1 - side]);
        let minute = self.t / 60.0;
        let shift = match diff {
            d if d < 0 && minute >= 80.0 => 2.0,
            d if d < 0 && minute >= 60.0 => 1.0,
            d if d >= 2 => -1.0,
            d if d > 0 && minute >= 70.0 => -1.0,
            _ => 0.0,
        };
        (base + shift).clamp(-2.0, 2.0)
    }

    /// Logit bonus for attacking near goal against `def`'s current
    /// mentality: attacking sides commit players forward and leave space
    /// behind; defensive sides crowd their own box.
    fn exposure(&self, def: usize, band: usize) -> f64 {
        if band >= 3 {
            self.p.exposure_effect * self.mentality(def)
        } else {
            0.0
        }
    }

    /// Logit penalty from the defending team's pressing in this band.
    fn pressure(&self, def: usize, band: usize) -> f64 {
        let press = f64::from(self.teams[def].tactic.pressing);
        if band <= 2 {
            self.p.press_effect * press
        } else if band >= 4 && press < 0.0 {
            -0.06 * press
        } else {
            0.0
        }
    }

    fn turnover(&mut self, to_side: usize, band: usize, lane: usize, carrier: Option<usize>) {
        let carrier = match carrier {
            Some(c) => c,
            None => self.pick(to_side, band, lane, true, None),
        };
        self.ball = Ball {
            side: to_side,
            band,
            lane,
            carrier,
            last_passer: None,
            beat_man: false,
        };
    }

    /// One action. Returns the seconds it took.
    fn step(&mut self) -> f64 {
        let att = self.ball.side;
        let def = 1 - att;
        if self.teams[att].player_in(self.ball.carrier).is_none() {
            let (b, l) = (self.ball.band, self.ball.lane);
            self.ball.carrier = self.pick(att, b, l, true, None);
        }
        let band = self.ball.band;
        let lane = self.ball.lane;
        let dband = BANDS - 1 - band;
        let dlane = LANES - 1 - lane;
        let def_slot = self.pick(def, dband, dlane, false, None);

        // Fouls away from the main contest (shirt pulls, late challenges).
        let def_i = self.teams[def].on[def_slot].expect("picked on pitch");
        let tendency = self.teams[def].players[def_i].skills.foul_tendency;
        let press = f64::from(self.teams[def].tactic.pressing);
        if self
            .rng
            .chance(self.p.foul_rate * (0.5 + tendency) * (1.0 + 0.1 * press))
        {
            return self.foul(def, def_slot);
        }
        if self.rng.chance(self.p.injury_rate) {
            self.injury(att, self.ball.carrier);
            if self.teams[att].player_in(self.ball.carrier).is_none() {
                return 30.0;
            }
        }

        let action = self.choose_action(att);
        let tempo = f64::from(self.teams[att].tactic.tempo);
        let time_mult = 1.0 - 0.08 * tempo;
        let (in_play, dead) = match action {
            Action::Short => self.short_pass(att, def, def_slot),
            Action::Long => self.long_pass(att, def, def_slot),
            Action::Through => self.through_ball(att, def, def_slot),
            Action::Cross => self.cross(att, def, def_slot),
            Action::Dribble => self.dribble(att, def, def_slot),
            Action::Shot => {
                let shooter = self.teams[att].on[self.ball.carrier].expect("carrier on pitch");
                let (base, kind) = if band == BANDS - 1 {
                    let xg = if lane == 1 {
                        self.p.xg_central[band]
                    } else {
                        self.p.xg_wide[band]
                    };
                    let bonus = if self.ball.beat_man { 1.35 } else { 1.0 };
                    (xg * bonus, ShotKind::Open)
                } else {
                    let xg = if lane == 1 {
                        self.p.xg_central[band]
                    } else {
                        self.p.xg_wide[band]
                    };
                    (xg, ShotKind::Long)
                };
                let assister = self.ball.last_passer;
                (2.0, self.shot(att, shooter, base, kind, assister))
            }
        };
        let in_play = in_play * time_mult;
        self.teams[att].in_play_secs += in_play;
        in_play + dead
    }

    fn choose_action(&mut self, att: usize) -> Action {
        let band = self.ball.band;
        let lane = self.ball.lane;
        let team = &self.teams[att];
        let carrier = &team.players[team.on[self.ball.carrier].expect("carrier")];
        let s = &carrier.skills;
        let t = &team.tactic;
        let m = self.mentality(att);
        let (w, d) = (f64::from(t.width), f64::from(t.directness));
        // Base weights by band: short, long, through, cross, dribble, shot.
        let base: [f64; 6] = match band {
            0 => [0.82, 0.16, 0.0, 0.0, 0.012, 0.0],
            1 => [0.83, 0.13, 0.008, 0.0, 0.018, 0.0],
            2 => [0.81, 0.09, 0.015, 0.01, 0.036, 0.0],
            3 => [0.76, 0.05, 0.022, 0.07, 0.036, 0.015],
            4 => [0.62, 0.02, 0.03, 0.22, 0.045, 0.025],
            _ => [0.42, 0.0, 0.015, 0.20, 0.05, self.p.box_shot_weight],
        };
        let wide = lane != 1;
        let shot_skill = if band == BANDS - 1 {
            s.shoot
        } else {
            s.long_shot
        };
        let mut weights = [
            base[0] * (1.0 - 0.12 * d),
            base[1] * (1.0 + self.p.direct_effect * d).max(0.1),
            base[2] * (1.0 + 0.1 * d + 0.1 * m).max(0.1) * (0.5 + s.through),
            base[3] * if wide { (1.0 + 0.3 * w).max(0.2) } else { 0.15 } * (0.5 + s.cross),
            base[4] * (1.0 + 0.05 * m) * (0.5 + s.dribble),
            base[5] * (1.0 + 0.15 * m).max(0.2) * (0.5 + shot_skill),
        ];
        if self.ball.carrier == 0 {
            // Goalkeepers only pass.
            weights[2..].fill(0.0);
        }
        match self.rng.weighted_index(&weights).unwrap_or(0) {
            0 => Action::Short,
            1 => Action::Long,
            2 => Action::Through,
            3 => Action::Cross,
            4 => Action::Dribble,
            _ => Action::Shot,
        }
    }

    fn short_pass(&mut self, att: usize, def: usize, def_slot: usize) -> (f64, f64) {
        let band = self.ball.band;
        let lane = self.ball.lane;
        let ci = self.teams[att].on[self.ball.carrier].expect("carrier");
        let di = self.teams[def].on[def_slot].expect("defender");
        // Direction first: forward passes gain ground but fail more often.
        let tempo = f64::from(self.teams[att].tactic.tempo);
        let direct = f64::from(self.teams[att].tactic.directness);
        let fwd = (self.p.forward_rate
            + 0.04 * self.mentality(att)
            + 0.03 * tempo
            + self.p.direct_forward * direct)
            .clamp(0.05, 0.6);
        let r = self.rng.unit_f64();
        let (new_band, direction_logit) = if r < fwd && band < BANDS - 1 {
            (band + 1, -self.p.forward_risk)
        } else if r < fwd + 0.12 && band > 0 {
            (band - 1, 0.6)
        } else {
            (band, 0.2)
        };
        let c = &self.teams[att].players[ci];
        let d = &self.teams[def].players[di];
        let skill = c.skills.pass_short * c.fat() - 0.85 * d.skills.defend * d.fat();
        let logit = self.p.short_base[band]
            + direction_logit
            + self.p.pass_slope * skill
            + self.home_logit(att)
            - self.pressure(def, band)
            - 0.05 * tempo
            + self.exposure(def, band);
        self.count_pass(att, ci);
        if self.rng.chance(math::logistic(logit)) {
            self.complete_pass(att, ci);
            let new_lane = self.shift_lane(lane, 0.35);
            let receiver = self.pick(att, new_band, new_lane, true, Some(self.ball.carrier));
            self.ball = Ball {
                side: att,
                band: new_band,
                lane: new_lane,
                carrier: receiver,
                last_passer: Some(ci),
                beat_man: false,
            };
            (self.p.pass_secs, 0.0)
        } else if self.rng.chance(0.25) {
            self.turnover(def, BANDS - 1 - band, LANES - 1 - lane, None);
            (self.p.pass_secs, 12.0)
        } else {
            // Most misplaced passes are loose balls; about a third are
            // clean interceptions worth recording.
            if self.rng.chance(0.3) {
                self.teams[def].stats.interceptions += 1;
                let ds = &mut self.teams[def].players[di].stats;
                ds.interceptions = ds.interceptions.saturating_add(1);
            }
            self.turnover(def, BANDS - 1 - band, LANES - 1 - lane, Some(def_slot));
            (self.p.pass_secs, 0.0)
        }
    }

    fn long_pass(&mut self, att: usize, def: usize, def_slot: usize) -> (f64, f64) {
        let band = self.ball.band;
        let lane = self.ball.lane;
        let ci = self.teams[att].on[self.ball.carrier].expect("carrier");
        let di = self.teams[def].on[def_slot].expect("defender");
        let c = &self.teams[att].players[ci];
        let d = &self.teams[def].players[di];
        let skill = c.skills.pass_long * c.fat() - 0.8 * d.skills.aerial_def * d.fat();
        let logit = self.p.long_base + self.p.skill_slope * skill + self.home_logit(att);
        self.count_pass(att, ci);
        if self.rng.chance(math::logistic(logit)) {
            self.complete_pass(att, ci);
            let new_band = (band + 2).min(4);
            let new_lane = self.rng.index(LANES);
            let receiver = self.pick(att, new_band, new_lane, true, Some(self.ball.carrier));
            self.ball = Ball {
                side: att,
                band: new_band,
                lane: new_lane,
                carrier: receiver,
                last_passer: Some(ci),
                beat_man: false,
            };
            (4.5, 0.0)
        } else if self.rng.chance(0.4) {
            let to_band = (BANDS - 1).saturating_sub((band + 2).min(BANDS - 1));
            let to_lane = self.rng.index(LANES);
            self.turnover(def, to_band, to_lane, None);
            (4.5, 14.0)
        } else {
            let to_band = (BANDS - 1).saturating_sub((band + 2).min(BANDS - 1));
            self.turnover(def, to_band, LANES - 1 - lane, None);
            (4.5, 0.0)
        }
    }

    fn through_ball(&mut self, att: usize, def: usize, def_slot: usize) -> (f64, f64) {
        let band = self.ball.band;
        let lane = self.ball.lane;
        let ci = self.teams[att].on[self.ball.carrier].expect("carrier");
        let receiver_slot = self.pick(att, BANDS - 1, 1, true, Some(self.ball.carrier));
        let ri = self.teams[att].on[receiver_slot].expect("receiver");
        let di = self.teams[def].on[def_slot].expect("defender");
        let c = &self.teams[att].players[ci];
        let r = &self.teams[att].players[ri];
        let d = &self.teams[def].players[di];
        let skill = 0.6 * c.skills.through * c.fat() + 0.4 * r.skills.off_ball * r.fat()
            - d.skills.defend * d.fat();
        let line = f64::from(self.teams[def].tactic.defensive_line);
        let logit = self.p.through_base
            + self.p.skill_slope * skill
            + 0.25 * line
            + self.home_logit(att)
            + 1.5 * self.exposure(def, BANDS - 1);
        self.count_pass(att, ci);
        if self.rng.chance(math::logistic(logit)) {
            self.complete_pass(att, ci);
            self.ball.carrier = receiver_slot;
            self.ball.band = BANDS - 1;
            self.ball.lane = 1;
            let dead = self.shot(att, ri, self.p.xg_big_chance, ShotKind::Big, Some(ci));
            (5.0, dead)
        } else if self.rng.chance((0.3 + 0.08 * line).clamp(0.05, 0.6)) {
            self.teams[att].stats.offsides += 1;
            self.turnover(def, 1, LANES - 1 - lane, None);
            (4.0, 20.0)
        } else {
            self.teams[def].stats.interceptions += 1;
            self.teams[def].players[di].stats.interceptions += 1;
            self.turnover(
                def,
                BANDS - 1 - band.max(3),
                LANES - 1 - lane,
                Some(def_slot),
            );
            (4.0, 0.0)
        }
    }

    fn cross(&mut self, att: usize, def: usize, def_slot: usize) -> (f64, f64) {
        let lane = self.ball.lane;
        let ci = self.teams[att].on[self.ball.carrier].expect("carrier");
        let di = self.teams[def].on[def_slot].expect("defender");
        let c = &self.teams[att].players[ci];
        let d = &self.teams[def].players[di];
        let skill = c.skills.cross * c.fat() - 0.7 * d.skills.aerial_def * d.fat();
        let width = f64::from(self.teams[att].tactic.width);
        let logit =
            self.p.cross_base + self.p.skill_slope * skill + 0.1 * width + self.home_logit(att);
        self.teams[att].stats.crosses += 1;
        if self.rng.chance(math::logistic(logit)) {
            self.teams[att].stats.crosses_completed += 1;
            // Whoever attacks the ball: box presence weighted by heading.
            let team = &self.teams[att];
            let mut w = team.attack_w[BANDS - 1][1];
            w[self.ball.carrier] = 0.0;
            for (s, weight) in w.iter_mut().enumerate() {
                if let Some(i) = team.on[s] {
                    *weight *= 0.3 + team.players[i].skills.header;
                }
            }
            let hs = self.rng.weighted_index(&w).unwrap_or(10);
            let Some(hi) = self.teams[att].on[hs] else {
                self.turnover(def, 0, 1, None);
                return (4.0, 0.0);
            };
            self.ball.carrier = hs;
            self.ball.band = BANDS - 1;
            self.ball.lane = 1;
            let dead = self.shot(att, hi, self.p.xg_header, ShotKind::Header, Some(ci));
            (4.0, dead)
        } else {
            let r = self.rng.unit_f64();
            if r < 0.25 {
                (4.0, self.corner(att))
            } else if r < 0.5 {
                self.turnover(def, 0, 1, None);
                (4.0, 20.0)
            } else {
                self.turnover(def, 1, LANES - 1 - lane, None);
                (4.0, 0.0)
            }
        }
    }

    fn dribble(&mut self, att: usize, def: usize, def_slot: usize) -> (f64, f64) {
        let band = self.ball.band;
        let lane = self.ball.lane;
        let ci = self.teams[att].on[self.ball.carrier].expect("carrier");
        let di = self.teams[def].on[def_slot].expect("defender");
        let c = &self.teams[att].players[ci];
        let d = &self.teams[def].players[di];
        let skill = c.skills.dribble * c.fat() - d.skills.tackle * d.fat();
        let tendency = d.skills.foul_tendency;
        let logit = self.p.dribble_base + self.p.skill_slope * skill + self.home_logit(att)
            - 0.5 * self.pressure(def, band);
        self.teams[att].stats.dribbles += 1;
        let cs = &mut self.teams[att].players[ci].stats;
        cs.dribbles = cs.dribbles.saturating_add(1);
        if self.rng.chance(math::logistic(logit)) {
            self.teams[att].stats.dribbles_won += 1;
            let cs = &mut self.teams[att].players[ci].stats;
            cs.dribbles_won = cs.dribbles_won.saturating_add(1);
            if band < BANDS - 1 {
                self.ball.band = band + 1;
                self.ball.lane = self.shift_lane(lane, 0.3);
            } else {
                self.ball.beat_man = true;
            }
            (3.5, 0.0)
        } else if self.rng.chance(self.p.dribble_foul * (0.5 + tendency)) {
            (3.5, self.foul(def, def_slot))
        } else {
            self.teams[def].stats.tackles += 1;
            let ds = &mut self.teams[def].players[di].stats;
            ds.tackles = ds.tackles.saturating_add(1);
            self.turnover(def, BANDS - 1 - band, LANES - 1 - lane, Some(def_slot));
            (3.5, 0.0)
        }
    }

    fn shift_lane(&mut self, lane: usize, p: f64) -> usize {
        if !self.rng.chance(p) {
            return lane;
        }
        match lane {
            0 => 1,
            2 => 1,
            _ => {
                if self.rng.chance(0.5) {
                    0
                } else {
                    2
                }
            }
        }
    }

    fn count_pass(&mut self, att: usize, ci: usize) {
        self.teams[att].stats.passes += 1;
        self.teams[att].players[ci].stats.passes += 1;
    }

    fn complete_pass(&mut self, att: usize, ci: usize) {
        self.teams[att].stats.passes_completed += 1;
        self.teams[att].players[ci].stats.passes_completed += 1;
    }

    /// A foul by the defender in `def_slot`. Returns dead-ball seconds.
    fn foul(&mut self, def: usize, def_slot: usize) -> f64 {
        let att = 1 - def;
        let minute = self.minute();
        let band = self.ball.band;
        let lane = self.ball.lane;
        let di = self.teams[def].on[def_slot].expect("defender");
        self.teams[def].stats.fouls += 1;
        let hot = self.teams[def].players[di].skills.hot_head;
        let ds = &mut self.teams[def].players[di].stats;
        ds.fouls = ds.fouls.saturating_add(1);
        let danger = if band >= 4 { 1.5 } else { 1.0 };
        let id = self.teams[def].players[di].snap.id;
        if self.rng.chance(self.p.straight_red * (0.5 + hot)) {
            self.send_off(def, def_slot, id, minute);
        } else if self.rng.chance(
            self.p.card_rate
                * (0.6 + 0.8 * hot)
                * danger
                // Booked players pull out of challenges; referees are
                // slower to show a second card.
                * if self.teams[def].players[di].stats.yellow { 0.25 } else { 1.0 },
        ) {
            let already = self.teams[def].players[di].stats.yellow;
            self.teams[def].players[di].stats.yellow = true;
            self.teams[def].stats.yellows += 1;
            self.events.push(MatchEvent::Card {
                minute,
                side: Side::from_index(def),
                player: id,
                red: false,
            });
            if already {
                self.send_off(def, def_slot, id, minute);
            }
        }
        // The free kick.
        if band == BANDS - 1 && self.rng.chance(0.35) {
            let taker = self.teams[att].best_by(|s| s.penalty);
            let taker_slot = self.teams[att].players[taker].slot.expect("on pitch");
            self.ball.carrier = taker_slot;
            return 30.0 + self.shot(att, taker, self.p.xg_penalty, ShotKind::Penalty, None);
        }
        if band == 4 && lane == 1 && self.rng.chance(0.35) {
            let taker = self.teams[att].best_by(|s| s.free_kick);
            let taker_slot = self.teams[att].players[taker].slot.expect("on pitch");
            self.ball.carrier = taker_slot;
            return 25.0 + self.shot(att, taker, self.p.xg_free_kick, ShotKind::FreeKick, None);
        }
        25.0
    }

    fn send_off(&mut self, side: usize, slot: usize, id: fs_core::ids::PlayerId, minute: u8) {
        let team = &mut self.teams[side];
        let i = team.on[slot].expect("on pitch");
        team.players[i].stats.red = true;
        team.players[i].slot = None;
        team.players[i].off_minute = Some(minute);
        team.on[slot] = None;
        team.stats.reds += 1;
        self.events.push(MatchEvent::Card {
            minute,
            side: Side::from_index(side),
            player: id,
            red: true,
        });
        if slot == 0 {
            self.replace_keeper(side);
        }
        self.teams[side].rebuild_weights();
    }

    /// After the goalkeeper leaves: bring on a bench keeper for the weakest
    /// outfielder if possible, otherwise move the best outfield keeper into
    /// goal.
    fn replace_keeper(&mut self, side: usize) {
        let team = &self.teams[side];
        let bench_gk = team
            .bench
            .iter()
            .position(|&b| team.players[b].snap.positions.best() == Position::Goalkeeper);
        let can_sub = team.subs_made < MAX_SUBS && team.windows_used < MAX_SUB_WINDOWS;
        let weakest = (1..11).filter(|&s| team.on[s].is_some()).min_by(|&a, &b| {
            let oa = team.players[team.on[a].expect("on")].skills.overall;
            let ob = team.players[team.on[b].expect("on")].skills.overall;
            oa.total_cmp(&ob).then(a.cmp(&b))
        });
        let Some(weakest) = weakest else { return };
        if let (Some(k), true) = (bench_gk, can_sub) {
            // Take the outfielder off, then put the keeper in slot 0.
            let minute = self.minute();
            let team = &mut self.teams[side];
            let out = team.on[weakest].expect("on");
            team.players[out].slot = None;
            team.players[out].off_minute = Some(minute);
            team.on[weakest] = None;
            let gk = team.bench.remove(k);
            let pl = &mut team.players[gk];
            pl.skills = Skills::of(pl.snap, Position::Goalkeeper);
            pl.slot = Some(0);
            pl.on_minute = minute;
            pl.stats.minutes = 1;
            pl.defensive = true;
            team.on[0] = Some(gk);
            team.subs_made += 1;
            team.windows_used += 1;
            let (off, on) = (team.players[out].snap.id, team.players[gk].snap.id);
            self.events.push(MatchEvent::Substitution {
                minute,
                side: Side::from_index(side),
                off,
                on,
            });
        } else {
            let team = &mut self.teams[side];
            let best = (1..11)
                .filter(|&s| team.on[s].is_some())
                .max_by(|&a, &b| {
                    let ka = Skills::of(
                        team.players[team.on[a].expect("on")].snap,
                        Position::Goalkeeper,
                    )
                    .keeper;
                    let kb = Skills::of(
                        team.players[team.on[b].expect("on")].snap,
                        Position::Goalkeeper,
                    )
                    .keeper;
                    ka.total_cmp(&kb).then(b.cmp(&a))
                })
                .unwrap_or(weakest);
            let i = team.on[best].take().expect("on");
            let pl = &mut team.players[i];
            pl.skills = Skills::of(pl.snap, Position::Goalkeeper);
            pl.slot = Some(0);
            team.on[0] = Some(i);
        }
    }

    fn injury(&mut self, side: usize, slot: usize) {
        let Some(i) = self.teams[side].on[slot] else {
            return;
        };
        let pl = &self.teams[side].players[i];
        let risk = (0.5 + pl.skills.injury_proneness) * (1.5 - pl.energy);
        if !self.rng.chance((risk / 1.5).min(1.0)) {
            return;
        }
        let minute = self.minute();
        let id = pl.snap.id;
        self.teams[side].players[i].stats.injured = true;
        self.events.push(MatchEvent::Injury {
            minute,
            side: Side::from_index(side),
            player: id,
        });
        let team = &self.teams[side];
        let can_sub = team.subs_made < MAX_SUBS
            && team.windows_used < MAX_SUB_WINDOWS
            && !team.bench.is_empty();
        if slot == 0 {
            let team = &mut self.teams[side];
            team.players[i].slot = None;
            team.players[i].off_minute = Some(minute);
            team.on[0] = None;
            self.replace_keeper(side);
            self.teams[side].rebuild_weights();
        } else if can_sub {
            if self.substitute(side, slot) {
                self.teams[side].windows_used += 1;
            }
        } else {
            let team = &mut self.teams[side];
            team.players[i].slot = None;
            team.players[i].off_minute = Some(minute);
            team.on[slot] = None;
            team.rebuild_weights();
        }
    }

    fn corner(&mut self, att: usize) -> f64 {
        let def = 1 - att;
        self.teams[att].stats.corners += 1;
        let taker = self.teams[att].best_by(|s| s.corner);
        let r = self.rng.unit_f64();
        if r < 0.3 {
            let team = &self.teams[att];
            let mut w = [0.0; 11];
            for (s, weight) in w.iter_mut().enumerate().skip(1) {
                if let Some(i) = team.on[s]
                    && i != taker
                {
                    *weight = 0.2 + team.players[i].skills.header;
                }
            }
            let Some(hs) = self.rng.weighted_index(&w) else {
                self.turnover(def, 1, 1, None);
                return 25.0;
            };
            let hi = self.teams[att].on[hs].expect("on");
            let delivery = self.teams[att].players[taker].skills.corner;
            self.ball.carrier = hs;
            self.ball.band = BANDS - 1;
            self.ball.lane = 1;
            25.0 + self.shot(
                att,
                hi,
                self.p.xg_corner_header * (0.7 + 0.6 * delivery),
                ShotKind::CornerHeader,
                Some(taker),
            )
        } else if r < 0.42 {
            let taker_slot = self.teams[att].players[taker].slot.expect("on pitch");
            let lane = if self.rng.chance(0.5) { 0 } else { 2 };
            self.ball = Ball {
                side: att,
                band: BANDS - 1,
                lane,
                carrier: taker_slot,
                last_passer: None,
                beat_man: false,
            };
            25.0
        } else {
            let to_lane = self.rng.index(LANES);
            self.turnover(def, 1, to_lane, None);
            25.0
        }
    }

    /// Resolves a shot. Returns dead-ball seconds that follow it.
    fn shot(
        &mut self,
        att: usize,
        shooter: usize,
        base_xg: f64,
        kind: ShotKind,
        assister: Option<usize>,
    ) -> f64 {
        let def = 1 - att;
        let minute = self.minute();
        let pl = &self.teams[att].players[shooter];
        let skill = match kind {
            ShotKind::Header | ShotKind::CornerHeader => pl.skills.header,
            ShotKind::Long => pl.skills.long_shot,
            ShotKind::Penalty => pl.skills.penalty,
            ShotKind::FreeKick => pl.skills.free_kick,
            ShotKind::Open | ShotKind::Big => pl.skills.shoot,
        } * pl.fat();
        let shooter_id = pl.snap.id;

        // Aerial and keeper resistance.
        let keeper_i = self.teams[def].on[0];
        let keeper = keeper_i.map_or(0.1, |k| {
            let kp = &self.teams[def].players[k];
            let s = match kind {
                ShotKind::Header | ShotKind::CornerHeader => {
                    0.5 * kp.skills.keeper + 0.5 * kp.skills.keeper_aerial
                }
                _ => kp.skills.keeper,
            };
            s * kp.fat()
        });
        // Set-piece specialists matter less than open-play finishing.
        let mut xg = match kind {
            ShotKind::Penalty => base_xg * (0.85 + 0.25 * skill),
            _ => base_xg * (0.55 + 0.9 * skill),
        };
        if matches!(kind, ShotKind::Header | ShotKind::CornerHeader) {
            let marker = self.pick(def, 0, 1, false, Some(0));
            if let Some(mi) = self.teams[def].on[marker] {
                xg *= 1.25 - 0.5 * self.teams[def].players[mi].skills.aerial_def;
            }
        }
        let xg = xg.clamp(0.005, 0.95);
        // xG assumes an average keeper; better keepers save more.
        let p_goal = (xg * (1.0 + 1.2 * (self.p.average_keeper - keeper))).clamp(0.003, 0.97);
        let p_on = (self.p.on_target_base + 0.25 * skill + 0.6 * xg)
            .max(p_goal + 0.04)
            .min(0.99);
        let block = match kind {
            ShotKind::Open => 0.18,
            ShotKind::Long => 0.24,
            ShotKind::FreeKick => 0.15,
            _ => 0.0,
        };

        let team = &mut self.teams[att];
        team.stats.shots += 1;
        team.stats.xg += xg as f32;
        let ps = &mut team.players[shooter].stats;
        ps.shots = ps.shots.saturating_add(1);
        ps.xg += xg as f32;
        if let Some(a) = assister
            && a != shooter
        {
            let s = &mut team.players[a].stats;
            s.key_passes = s.key_passes.saturating_add(1);
        }

        if self.rng.chance(block) {
            if self.rng.chance(0.4) {
                return self.corner(att);
            }
            self.turnover(def, 1, 1, None);
            return 0.0;
        }
        // xG covers blocked shots too, so unblocked shots score a little
        // more often than their xG.
        let p_goal = (p_goal / (1.0 - block)).min(0.97);
        let p_on = p_on.max(p_goal + 0.04).min(0.99);
        let u = self.rng.unit_f64();
        if u < p_goal {
            let team = &mut self.teams[att];
            team.stats.shots_on_target += 1;
            let ps = &mut team.players[shooter].stats;
            ps.shots_on_target = ps.shots_on_target.saturating_add(1);
            ps.goals = ps.goals.saturating_add(1);
            let assist = match assister {
                Some(a) if a != shooter && kind != ShotKind::Penalty => {
                    let s = &mut team.players[a].stats;
                    s.assists = s.assists.saturating_add(1);
                    Some(team.players[a].snap.id)
                }
                _ => None,
            };
            self.goals[att] += 1;
            let conceding = &mut self.teams[def];
            for s in 0..11 {
                if let Some(i) = conceding.on[s] {
                    let st = &mut conceding.players[i].stats;
                    st.goals_conceded = st.goals_conceded.saturating_add(1);
                }
            }
            self.events.push(MatchEvent::Goal {
                minute,
                side: Side::from_index(att),
                scorer: shooter_id,
                assist,
                kind: match kind {
                    ShotKind::Open | ShotKind::Big => GoalKind::OpenPlay,
                    ShotKind::Header => GoalKind::Header,
                    ShotKind::CornerHeader => GoalKind::Corner,
                    ShotKind::Long => GoalKind::LongShot,
                    ShotKind::Penalty => GoalKind::Penalty,
                    ShotKind::FreeKick => GoalKind::FreeKick,
                },
                xg: xg as f32,
            });
            self.kickoff(def);
            return 60.0;
        }
        if kind == ShotKind::Penalty {
            self.events.push(MatchEvent::PenaltyMissed {
                minute,
                side: Side::from_index(att),
                taker: shooter_id,
            });
        }
        if u < p_on {
            self.teams[att].stats.shots_on_target += 1;
            let ps = &mut self.teams[att].players[shooter].stats;
            ps.shots_on_target = ps.shots_on_target.saturating_add(1);
            self.teams[def].stats.saves += 1;
            if let Some(k) = keeper_i {
                let ks = &mut self.teams[def].players[k].stats;
                ks.saves = ks.saves.saturating_add(1);
            }
            if self.rng.chance(0.3) {
                return self.corner(att);
            }
            self.turnover(def, 0, 1, Some(0));
            return 6.0;
        }
        self.turnover(def, 0, 1, Some(0));
        22.0
    }

    fn finish(mut self) -> MatchResult {
        let total_play = (self.teams[0].in_play_secs + self.teams[1].in_play_secs).max(1.0);
        let mut defensive = [Vec::new(), Vec::new()];
        let mut players = [Vec::new(), Vec::new()];
        let mut stats = [TeamStats::default(); 2];
        for side in 0..2 {
            let team = &mut self.teams[side];
            team.stats.possession = (team.in_play_secs / total_play) as f32;
            stats[side] = team.stats;
            for pl in &mut team.players {
                if pl.stats.started || pl.stats.minutes > 0 {
                    let off = pl.off_minute.unwrap_or(90);
                    // Anyone who appeared gets at least a minute.
                    pl.stats.minutes = off.saturating_sub(pl.on_minute).max(1);
                    pl.stats.energy = pl.energy as f32;
                }
                defensive[side].push(pl.defensive);
                players[side].push(pl.stats.clone());
            }
        }
        self.events.sort_by_key(MatchEvent::minute);
        let mut result = MatchResult {
            goals: self.goals,
            stats,
            players,
            events: self.events,
        };
        rate_players(&mut result, &defensive);
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::input;

    #[test]
    fn plays_a_full_match() {
        let r = play(&input(600, 600, 1));
        assert!(r.goals[0] < 15 && r.goals[1] < 15);
        let s = &r.stats;
        assert!(s[0].passes > 150 && s[1].passes > 150, "{s:?}");
        assert!((s[0].possession + s[1].possession - 1.0).abs() < 1e-3);
        let goals_from_events = r
            .events
            .iter()
            .filter(|e| matches!(e, MatchEvent::Goal { .. }))
            .count();
        assert_eq!(goals_from_events, usize::from(r.goals[0] + r.goals[1]));
        for side in &r.players {
            let scored: u32 = side.iter().map(|p| u32::from(p.goals)).sum();
            let _ = scored;
            assert!(side.iter().filter(|p| p.started).count() == 11);
        }
    }

    #[test]
    fn same_input_same_result() {
        let i = input(600, 550, 7);
        assert_eq!(play(&i), play(&i));
    }

    #[test]
    fn better_team_wins_more() {
        let mut strong = 0;
        let mut weak = 0;
        for k in 0..200 {
            let r = play(&input(750, 450, k));
            match r.outcome() {
                1 => strong += 1,
                -1 => weak += 1,
                _ => {}
            }
        }
        assert!(strong > weak * 3, "strong {strong} weak {weak}");
    }

    #[test]
    fn substitutions_happen() {
        let r = play(&input(600, 600, 3));
        let subs = r
            .events
            .iter()
            .filter(|e| matches!(e, MatchEvent::Substitution { .. }))
            .count();
        assert!(subs >= 1);
    }

    #[test]
    fn minutes_add_up() {
        for k in 0..100 {
            let r = play(&input(600, 580, k));
            for side in 0..2 {
                let total: u32 = r.players[side].iter().map(|p| u32::from(p.minutes)).sum();
                let reds = u32::from(r.stats[side].reds);
                assert!(total <= 11 * 90 + 5, "{total}");
                assert!(
                    total + reds * 90 + 90 >= 11 * 90,
                    "{total} with {reds} reds"
                );
                for p in &r.players[side] {
                    assert!(p.minutes <= 90);
                    assert!((3.0..=10.0).contains(&p.rating));
                    if !p.started && p.minutes > 0 {
                        assert!(r.events.iter().any(
                            |e| matches!(e, MatchEvent::Substitution { on, .. } if *on == p.player)
                        ));
                    }
                }
            }
        }
    }

    #[test]
    fn attacking_mentality_takes_more_shots() {
        let mut shots = [0u32; 2];
        for k in 0..300 {
            let mut i = input(600, 600, k);
            i.home.tactic.mentality = 2;
            let r = play(&i);
            shots[0] += u32::from(r.stats[0].shots);
            let mut j = input(600, 600, k);
            j.home.tactic.mentality = -2;
            shots[1] += u32::from(play(&j).stats[0].shots);
        }
        assert!(shots[0] > shots[1] * 11 / 10, "{shots:?}");
    }

    #[test]
    fn rare_events_happen_at_sane_rates() {
        let (mut reds, mut injuries, mut pens) = (0, 0, 0);
        let n = 500;
        for k in 0..n {
            let r = play(&input(600, 600, k));
            reds += r.stats[0].reds + r.stats[1].reds;
            for e in &r.events {
                match e {
                    MatchEvent::Injury { .. } => injuries += 1,
                    MatchEvent::Goal {
                        kind: GoalKind::Penalty,
                        ..
                    }
                    | MatchEvent::PenaltyMissed { .. } => pens += 1,
                    _ => {}
                }
            }
        }
        let per = |x: u32| f64::from(x) / f64::from(n as u32);
        assert!(
            (0.03..0.4).contains(&per(u32::from(reds))),
            "reds {}",
            per(u32::from(reds))
        );
        assert!(
            (0.05..1.0).contains(&per(injuries)),
            "injuries {}",
            per(injuries)
        );
        assert!((0.05..0.6).contains(&per(pens)), "penalties {}", per(pens));
    }

    #[test]
    fn neutral_venue_removes_home_edge() {
        let (mut home, mut away) = (0, 0);
        for k in 0..400 {
            let mut i = input(600, 600, k);
            i.neutral = true;
            match play(&i).outcome() {
                1 => home += 1,
                -1 => away += 1,
                _ => {}
            }
        }
        assert!(
            (home as f64 / away.max(1) as f64 - 1.0).abs() < 0.3,
            "{home} {away}"
        );
    }
}
