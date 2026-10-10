//! Resolving the ball carrier's actions: passes, dribbles, crosses, shots,
//! fouls, set pieces, cards and injuries.

use super::engine::{Action, Ball, Engine, ShotKind};
use super::{BANDS, LANES, MAX_SUB_WINDOWS, MAX_SUBS};
use crate::instruction::Shout;
use crate::log::{ActionKind, Outcome, Zone};
use crate::result::{GoalKind, MatchEvent, Side};
use crate::skills::Skills;
use fs_core::attributes::Position;
use fs_core::math;

impl Engine {
    /// Resolves one action. Returns (seconds in play, dead-ball seconds).
    pub(super) fn step(&mut self) -> (f64, f64) {
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
        let calm = if self.teams[def].shout == Some(Shout::CalmDown) {
            0.6
        } else {
            1.0
        };
        if self
            .rng
            .chance(self.p.foul_rate * (0.5 + tendency) * (1.0 + 0.1 * press) * calm)
        {
            return (0.0, self.foul(def, def_slot));
        }
        if self.rng.chance(self.p.injury_rate) {
            self.injury(att, self.ball.carrier);
            if self.teams[att].player_in(self.ball.carrier).is_none() {
                return (0.0, 30.0);
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
        (in_play, dead)
    }

    /// The ball's zone in `side`'s frame.
    pub(super) fn here_for(&self, side: usize) -> Zone {
        let z = self.here();
        if side == self.ball.side {
            z
        } else {
            Zone {
                band: (BANDS - 1) as u8 - z.band,
                lane: (LANES - 1) as u8 - z.lane,
            }
        }
    }

    pub(super) fn choose_action(&mut self, att: usize) -> Action {
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

    pub(super) fn short_pass(&mut self, att: usize, def: usize, def_slot: usize) -> (f64, f64) {
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
            let ri = self.teams[att].on[receiver];
            let to = zone(new_band, new_lane);
            self.note(
                att,
                ActionKind::ShortPass,
                Outcome::Complete,
                ci,
                ri,
                None,
                to,
                0.0,
            );
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
            let here = self.here();
            self.note(
                att,
                ActionKind::ShortPass,
                Outcome::OutOfPlay,
                ci,
                None,
                Some(di),
                here,
                0.0,
            );
            self.turnover(def, BANDS - 1 - band, LANES - 1 - lane, None);
            (self.p.pass_secs, 12.0)
        } else {
            let here = self.here();
            self.note(
                att,
                ActionKind::ShortPass,
                Outcome::Intercepted,
                ci,
                None,
                Some(di),
                here,
                0.0,
            );
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

    pub(super) fn long_pass(&mut self, att: usize, def: usize, def_slot: usize) -> (f64, f64) {
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
            let ri = self.teams[att].on[receiver];
            self.note(
                att,
                ActionKind::LongPass,
                Outcome::Complete,
                ci,
                ri,
                None,
                zone(new_band, new_lane),
                0.0,
            );
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
            let landing = zone((band + 2).min(BANDS - 1), LANES - 1 - to_lane);
            self.note(
                att,
                ActionKind::LongPass,
                Outcome::OutOfPlay,
                ci,
                None,
                None,
                landing,
                0.0,
            );
            self.turnover(def, to_band, to_lane, None);
            (4.5, 14.0)
        } else {
            let to_band = (BANDS - 1).saturating_sub((band + 2).min(BANDS - 1));
            let landing = zone((band + 2).min(BANDS - 1), lane);
            self.note(
                att,
                ActionKind::LongPass,
                Outcome::Intercepted,
                ci,
                None,
                Some(di),
                landing,
                0.0,
            );
            self.turnover(def, to_band, LANES - 1 - lane, None);
            (4.5, 0.0)
        }
    }

    pub(super) fn through_ball(&mut self, att: usize, def: usize, def_slot: usize) -> (f64, f64) {
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
            self.note(
                att,
                ActionKind::ThroughBall,
                Outcome::Complete,
                ci,
                Some(ri),
                None,
                zone(BANDS - 1, 1),
                0.0,
            );
            self.ball.carrier = receiver_slot;
            self.ball.band = BANDS - 1;
            self.ball.lane = 1;
            let dead = self.shot(att, ri, self.p.xg_big_chance, ShotKind::Big, Some(ci));
            (5.0, dead)
        } else if self.rng.chance((0.3 + 0.08 * line).clamp(0.05, 0.6)) {
            self.note(
                att,
                ActionKind::ThroughBall,
                Outcome::Offside,
                ci,
                Some(ri),
                None,
                zone(BANDS - 1, lane),
                0.0,
            );
            self.teams[att].stats.offsides += 1;
            self.turnover(def, 1, LANES - 1 - lane, None);
            (4.0, 20.0)
        } else {
            let to = zone((band + 1).min(BANDS - 1), lane);
            self.note(
                att,
                ActionKind::ThroughBall,
                Outcome::Intercepted,
                ci,
                Some(ri),
                Some(di),
                to,
                0.0,
            );
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

    pub(super) fn cross(&mut self, att: usize, def: usize, def_slot: usize) -> (f64, f64) {
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
            self.note(
                att,
                ActionKind::Cross,
                Outcome::Complete,
                ci,
                Some(hi),
                None,
                zone(BANDS - 1, 1),
                0.0,
            );
            self.ball.carrier = hs;
            self.ball.band = BANDS - 1;
            self.ball.lane = 1;
            let dead = self.shot(att, hi, self.p.xg_header, ShotKind::Header, Some(ci));
            (4.0, dead)
        } else {
            let r = self.rng.unit_f64();
            let to = zone(BANDS - 1, 1);
            if r < 0.25 {
                self.note(
                    att,
                    ActionKind::Cross,
                    Outcome::Blocked,
                    ci,
                    None,
                    Some(di),
                    to,
                    0.0,
                );
                (4.0, self.corner(att))
            } else if r < 0.5 {
                self.note(
                    att,
                    ActionKind::Cross,
                    Outcome::OutOfPlay,
                    ci,
                    None,
                    None,
                    to,
                    0.0,
                );
                self.turnover(def, 0, 1, None);
                (4.0, 20.0)
            } else {
                self.note(
                    att,
                    ActionKind::Cross,
                    Outcome::Intercepted,
                    ci,
                    None,
                    Some(di),
                    to,
                    0.0,
                );
                self.turnover(def, 1, LANES - 1 - lane, None);
                (4.0, 0.0)
            }
        }
    }

    pub(super) fn dribble(&mut self, att: usize, def: usize, def_slot: usize) -> (f64, f64) {
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
                let new_lane = self.shift_lane(lane, 0.3);
                self.note(
                    att,
                    ActionKind::Dribble,
                    Outcome::Complete,
                    ci,
                    None,
                    Some(di),
                    zone(band + 1, new_lane),
                    0.0,
                );
                self.ball.band = band + 1;
                self.ball.lane = new_lane;
            } else {
                let here = self.here();
                self.note(
                    att,
                    ActionKind::Dribble,
                    Outcome::Complete,
                    ci,
                    None,
                    Some(di),
                    here,
                    0.0,
                );
                self.ball.beat_man = true;
            }
            (3.5, 0.0)
        } else if self.rng.chance(self.p.dribble_foul * (0.5 + tendency)) {
            let here = self.here();
            self.note(
                att,
                ActionKind::Dribble,
                Outcome::Foul,
                ci,
                None,
                Some(di),
                here,
                0.0,
            );
            (3.5, self.foul(def, def_slot))
        } else {
            let here = self.here();
            self.note(
                att,
                ActionKind::Dribble,
                Outcome::Tackled,
                ci,
                None,
                Some(di),
                here,
                0.0,
            );
            self.teams[def].stats.tackles += 1;
            let ds = &mut self.teams[def].players[di].stats;
            ds.tackles = ds.tackles.saturating_add(1);
            self.turnover(def, BANDS - 1 - band, LANES - 1 - lane, Some(def_slot));
            (3.5, 0.0)
        }
    }

    pub(super) fn shift_lane(&mut self, lane: usize, p: f64) -> usize {
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

    pub(super) fn count_pass(&mut self, att: usize, ci: usize) {
        self.teams[att].stats.passes += 1;
        self.teams[att].players[ci].stats.passes += 1;
    }

    pub(super) fn complete_pass(&mut self, att: usize, ci: usize) {
        self.teams[att].stats.passes_completed += 1;
        self.teams[att].players[ci].stats.passes_completed += 1;
    }

    /// A foul by the defender in `def_slot`. Returns dead-ball seconds.
    pub(super) fn foul(&mut self, def: usize, def_slot: usize) -> f64 {
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
        let victim = self.teams[att].on[self.ball.carrier];
        let spot = self.here_for(def);
        self.note(
            def,
            ActionKind::Foul,
            Outcome::Foul,
            di,
            None,
            victim,
            spot,
            0.0,
        );
        let calm = if self.teams[def].shout == Some(Shout::CalmDown) {
            0.7
        } else {
            1.0
        };
        if self.rng.chance(self.p.straight_red * (0.5 + hot)) {
            self.send_off(def, def_slot, id, minute);
        } else if self.rng.chance(
            self.p.card_rate
                * (0.6 + 0.8 * hot)
                * danger
                // Booked players pull out of challenges; referees are
                // slower to show a second card.
                * if self.teams[def].players[di].stats.yellow { 0.25 } else { 1.0 }
                * calm,
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

    pub(super) fn send_off(
        &mut self,
        side: usize,
        slot: usize,
        id: fs_core::ids::PlayerId,
        minute: u8,
    ) {
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
    pub(super) fn replace_keeper(&mut self, side: usize) {
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
            pl.skills = Skills::of(&pl.snap, Position::Goalkeeper);
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
                        &team.players[team.on[a].expect("on")].snap,
                        Position::Goalkeeper,
                    )
                    .keeper;
                    let kb = Skills::of(
                        &team.players[team.on[b].expect("on")].snap,
                        Position::Goalkeeper,
                    )
                    .keeper;
                    ka.total_cmp(&kb).then(b.cmp(&a))
                })
                .unwrap_or(weakest);
            let i = team.on[best].take().expect("on");
            let pl = &mut team.players[i];
            pl.skills = Skills::of(&pl.snap, Position::Goalkeeper);
            pl.slot = Some(0);
            team.on[0] = Some(i);
        }
    }

    pub(super) fn injury(&mut self, side: usize, slot: usize) {
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

    pub(super) fn corner(&mut self, att: usize) -> f64 {
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
            self.note_from(
                zone(BANDS - 1, 0),
                att,
                ActionKind::Corner,
                Outcome::Complete,
                taker,
                Some(hi),
                None,
                zone(BANDS - 1, 1),
                0.0,
            );
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
            self.note_from(
                zone(BANDS - 1, lane),
                att,
                ActionKind::Corner,
                Outcome::Complete,
                taker,
                None,
                None,
                zone(BANDS - 1, lane),
                0.0,
            );
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
            self.note_from(
                zone(BANDS - 1, 0),
                att,
                ActionKind::Corner,
                Outcome::Cleared,
                taker,
                None,
                None,
                zone(BANDS - 1, 1),
                0.0,
            );
            self.turnover(def, 1, to_lane, None);
            25.0
        }
    }

    /// Resolves a shot. Returns dead-ball seconds that follow it.
    pub(super) fn shot(
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

        let shot_kind = match kind {
            ShotKind::Open | ShotKind::Long | ShotKind::Big => ActionKind::Shot,
            ShotKind::Header | ShotKind::CornerHeader => ActionKind::Header,
            ShotKind::Penalty => ActionKind::Penalty,
            ShotKind::FreeKick => ActionKind::FreeKick,
        };
        let goalmouth = zone(BANDS - 1, 1);
        if self.rng.chance(block) {
            self.note(
                att,
                shot_kind,
                Outcome::Blocked,
                shooter,
                None,
                None,
                goalmouth,
                xg,
            );
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
        let outcome = if u < p_goal {
            Outcome::Goal
        } else if u < p_on {
            Outcome::Saved
        } else {
            Outcome::Missed
        };
        self.note(
            att, shot_kind, outcome, shooter, None, keeper_i, goalmouth, xg,
        );
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

    /// Notes an action starting from the ball's zone in `side`'s frame.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn note(
        &mut self,
        side: usize,
        kind: ActionKind,
        outcome: Outcome,
        actor: usize,
        target: Option<usize>,
        opponent: Option<usize>,
        to: Zone,
        xg: f64,
    ) {
        if self.log.is_some() {
            let from = self.here_for(side);
            self.note_from(from, side, kind, outcome, actor, target, opponent, to, xg);
        }
    }
}

fn zone(band: usize, lane: usize) -> Zone {
    Zone {
        band: band as u8,
        lane: lane as u8,
    }
}
