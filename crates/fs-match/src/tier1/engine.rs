//! The tier 1 engine: match state, the clock, substitutions and the final
//! result. Action resolution lives in `actions.rs`.

use super::params::Params;
use super::team::{Team, is_defensive};
use super::{HALF_SECS, MAX_SUB_WINDOWS, MAX_SUBS};
use crate::input::{Decider, MatchInput, Tactic};
use crate::instruction::{Instruction, InstructionError, Shout, TimedInstruction};
use crate::log::{ActionKind, ActionRecord, Outcome, Zone};
use crate::result::{MatchEvent, MatchResult, Side, TeamStats, rate_players};
use crate::shootout::{self, Shootout};
use crate::skills::Skills;
use fs_core::Rng;

#[derive(Debug, Clone, Copy)]
pub(super) struct Ball {
    pub(super) side: usize,
    pub(super) band: usize,
    pub(super) lane: usize,
    /// Slot of the player on the ball.
    pub(super) carrier: usize,
    /// Player index of the last successful passer in this possession.
    pub(super) last_passer: Option<usize>,
    /// The carrier just beat a defender in the box.
    pub(super) beat_man: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Action {
    Short,
    Long,
    Through,
    Cross,
    Dribble,
    Shot,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ShotKind {
    Open,
    Long,
    Big,
    Header,
    CornerHeader,
    Penalty,
    FreeKick,
}

pub(super) struct Engine {
    pub(super) p: Params,
    pub(super) rng: Rng,
    pub(super) teams: [Team; 2],
    pub(super) ball: Ball,
    pub(super) t: f64,
    pub(super) half: u8,
    pub(super) neutral: bool,
    pub(super) goals: [u8; 2],
    pub(super) events: Vec<MatchEvent>,
    /// End of the current half (match clock seconds).
    pub(super) end: f64,
    pub(super) next_minute: f64,
    pub(super) stoppage2: f64,
    /// Real length of the first half, including stoppage time.
    pub(super) first_half_len: f64,
    /// Seconds since kickoff when the current half began.
    pub(super) half_offset: f64,
    /// Match clock when the current half began.
    pub(super) half_start_clock: f64,
    /// Seconds since kickoff when each half after the first began.
    pub(super) half_starts: Vec<f64>,
    /// How a level knockout tie is settled, if this is one.
    pub(super) decider: Option<Decider>,
    /// Extra substitutions (and windows) allowed in extra time.
    pub(super) extra_subs: u8,
    pub(super) shootout: Option<Shootout>,
    /// Engine steps completed.
    pub(super) steps: u32,
    pub(super) finished: bool,
    /// Instructions to apply at given steps (a replayed live match).
    pub(super) scheduled: Vec<TimedInstruction>,
    pub(super) next_scheduled: usize,
    /// Every instruction applied so far.
    pub(super) applied: Vec<TimedInstruction>,
    /// Action records, when recording.
    pub(super) log: Option<Vec<ActionRecord>>,
    /// Actions noted during the current step, before timing is known.
    pub(super) notes: Vec<Note>,
}

/// An action noted mid-step; timed when the step completes.
#[derive(Debug, Clone, Copy)]
pub(super) struct Note {
    pub(super) side: usize,
    pub(super) kind: ActionKind,
    pub(super) outcome: Outcome,
    pub(super) actor: usize,
    pub(super) target: Option<usize>,
    pub(super) opponent: Option<usize>,
    pub(super) from: Zone,
    pub(super) to: Zone,
    pub(super) xg: f32,
}

impl Engine {
    pub(super) fn new(input: &MatchInput, p: &Params, record: bool) -> Self {
        let mut e = Self {
            p: p.clone(),
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
            end: 0.0,
            next_minute: 60.0,
            stoppage2: 0.0,
            first_half_len: HALF_SECS,
            half_offset: 0.0,
            half_start_clock: 0.0,
            half_starts: Vec::new(),
            decider: input.decider,
            extra_subs: 0,
            shootout: None,
            steps: 0,
            finished: false,
            scheduled: input.instructions.clone(),
            next_scheduled: 0,
            applied: Vec::new(),
            log: record.then(|| Vec::with_capacity(1600)),
            notes: Vec::new(),
        };
        e.scheduled.sort_by_key(|i| i.step);
        e.kickoff(0);
        let stoppage1 = 60.0 + f64::from(e.rng.below(150));
        e.stoppage2 = 120.0 + f64::from(e.rng.below(300));
        e.end = HALF_SECS + stoppage1;
        e
    }

    /// Seconds since kickoff, counting first-half stoppage time.
    pub(super) fn elapsed(&self) -> f64 {
        if self.half == 0 {
            self.t
        } else {
            self.half_offset + (self.t - self.half_start_clock)
        }
    }

    /// Runs one engine step. Returns `false` once the match is over.
    pub(super) fn advance(&mut self) -> bool {
        if self.finished {
            return false;
        }
        self.apply_scheduled();
        self.expire_shouts();
        if self.t >= self.end {
            if self.half == 0 {
                self.first_half_len = self.t;
                self.half_offset = self.t;
                self.half_start_clock = HALF_SECS;
                self.half_starts.push(self.t);
                self.half = 1;
                self.t = HALF_SECS;
                self.next_minute = HALF_SECS + 60.0;
                self.end = 2.0 * HALF_SECS + self.stoppage2;
                self.half_time_recovery();
                self.kickoff(1);
                self.steps += 1;
                return true;
            }
            if self.half == 2 || (self.half == 1 && self.extra_time_due()) {
                self.start_extra_half();
                self.steps += 1;
                return true;
            }
            self.finished = true;
            self.settle();
            return false;
        }
        while self.t >= self.next_minute {
            self.minute_tick(self.next_minute);
            self.next_minute += 60.0;
        }
        let start = self.elapsed();
        let (in_play, dead) = self.step();
        self.t += in_play + dead;
        self.flush_notes(start, in_play, dead);
        self.steps += 1;
        true
    }

    /// Whether a level knockout tie goes to extra time now.
    fn extra_time_due(&self) -> bool {
        self.decider
            .is_some_and(|d| d.extra_time && d.level(self.goals))
    }

    /// Starts a 15-minute half of extra time.
    fn start_extra_half(&mut self) {
        let len = self.elapsed();
        self.half += 1;
        self.t = if self.half == 2 { 90.0 } else { 105.0 } * 60.0;
        self.half_start_clock = self.t;
        self.half_offset = len;
        self.half_starts.push(len);
        self.next_minute = self.t + 60.0;
        let stoppage = 30.0 + f64::from(self.rng.below(90));
        self.end = self.t + 15.0 * 60.0 + stoppage;
        let fresh = if self.half == 2 { 0.03 } else { 0.01 };
        for team in &mut self.teams {
            for p in &mut team.players {
                p.energy = (p.energy + fresh).min(1.0);
            }
        }
        if self.half == 2 {
            // One more substitution, in one more window.
            self.extra_subs = 1;
        }
        for side in 0..2 {
            if self.teams[side].auto_subs {
                let threshold = self.p.sub_energy + if self.half == 2 { 0.12 } else { 0.1 };
                self.ai_substitutions(side, 2, threshold);
            }
        }
        self.kickoff(usize::from(self.half % 2));
    }

    /// At the final whistle: penalties if the tie is still level.
    fn settle(&mut self) {
        let Some(d) = self.decider else { return };
        if !d.level(self.goals) {
            return;
        }
        let lineups = [0, 1].map(|side| {
            let team = &self.teams[side];
            let on: Vec<_> = (0..11)
                .filter_map(|s| team.on[s])
                .map(|i| (&team.players[i].snap, team.players[i].fat()))
                .collect();
            shootout::Lineup::new(&on)
        });
        let first = if self.rng.chance(0.5) {
            Side::Home
        } else {
            Side::Away
        };
        self.shootout = Some(shootout::play(
            &mut self.rng,
            [&lineups[0], &lineups[1]],
            first,
        ));
    }

    /// Substitutions allowed, including extra time's.
    pub(super) fn max_subs(&self) -> u8 {
        MAX_SUBS + self.extra_subs
    }

    /// Substitution windows allowed, including extra time's.
    fn max_windows(&self) -> u8 {
        MAX_SUB_WINDOWS + self.extra_subs
    }

    /// Plays to the end and returns the result.
    pub(super) fn run(mut self) -> MatchResult {
        while self.advance() {}
        self.finish()
    }

    /// Plays to the end and returns the result with the action log.
    pub(super) fn run_recorded(mut self) -> (MatchResult, Vec<ActionRecord>) {
        while self.advance() {}
        let log = self.log.take().unwrap_or_default();
        (self.finish(), log)
    }

    /// Notes an action for the log (no-op unless recording).
    #[allow(clippy::too_many_arguments)]
    pub(super) fn note_from(
        &mut self,
        from: Zone,
        side: usize,
        kind: ActionKind,
        outcome: Outcome,
        actor: usize,
        target: Option<usize>,
        opponent: Option<usize>,
        to: Zone,
        xg: f64,
    ) {
        if self.log.is_none() {
            return;
        }
        self.notes.push(Note {
            side,
            kind,
            outcome,
            actor,
            target,
            opponent,
            from,
            to,
            xg: xg as f32,
        });
    }

    /// The ball's current zone, for notes.
    pub(super) fn here(&self) -> Zone {
        Zone {
            band: self.ball.band as u8,
            lane: self.ball.lane as u8,
        }
    }

    fn flush_notes(&mut self, start: f64, in_play: f64, dead: f64) {
        let Some(log) = self.log.as_mut() else { return };
        let n = self.notes.len();
        if n == 0 {
            return;
        }
        let each = in_play / n as f64;
        for (k, note) in self.notes.drain(..).enumerate() {
            let id = |side: usize, i: usize| self.teams[side].players[i].snap.id;
            let other = 1 - note.side;
            log.push(ActionRecord {
                step: self.steps,
                at: (start + each * k as f64) as f32,
                duration: each as f32,
                dead: if k + 1 == n { dead as f32 } else { 0.0 },
                side: Side::from_index(note.side),
                kind: note.kind,
                outcome: note.outcome,
                actor: id(note.side, note.actor),
                target: note.target.map(|t| id(note.side, t)),
                opponent: note.opponent.map(|o| id(other, o)),
                from: note.from,
                to: note.to,
                xg: note.xg,
            });
        }
    }

    fn apply_scheduled(&mut self) {
        while let Some(&ins) = self.scheduled.get(self.next_scheduled) {
            if ins.step > self.steps {
                break;
            }
            self.next_scheduled += 1;
            // A replay only contains instructions that worked live.
            let _ = self.apply_instruction(ins.side, ins.instruction);
        }
    }

    /// Applies an instruction now and records it against the current step.
    pub(super) fn instruct(
        &mut self,
        side: Side,
        instruction: Instruction,
    ) -> Result<TimedInstruction, InstructionError> {
        if self.finished {
            return Err(InstructionError::MatchOver);
        }
        self.apply_instruction(side, instruction)?;
        let timed = TimedInstruction {
            step: self.steps,
            side,
            instruction,
        };
        Ok(timed)
    }

    fn apply_instruction(
        &mut self,
        side: Side,
        instruction: Instruction,
    ) -> Result<(), InstructionError> {
        let s = side.index();
        match instruction {
            Instruction::Substitute { off, on } => {
                let team = &self.teams[s];
                let slot = (0..11)
                    .find(|&k| team.on[k].is_some_and(|i| team.players[i].snap.id == off))
                    .ok_or(InstructionError::NotOnPitch(off))?;
                let k = team
                    .bench
                    .iter()
                    .position(|&b| team.players[b].snap.id == on)
                    .ok_or(InstructionError::NotOnBench(on))?;
                let minute = self.minute();
                let new_window = team.last_manual_sub != Some(minute);
                if team.subs_made >= self.max_subs()
                    || (new_window && team.windows_used >= self.max_windows())
                {
                    return Err(InstructionError::NoSubstitutionsLeft);
                }
                if new_window {
                    self.teams[s].windows_used += 1;
                    self.teams[s].last_manual_sub = Some(minute);
                }
                self.bring_on(s, slot, k);
            }
            Instruction::ChangeTactic(tactic) => self.change_tactic(s, tactic),
            Instruction::Shout(shout) => {
                let until = self.elapsed() + 15.0 * 60.0;
                let team = &mut self.teams[s];
                team.shout = Some(shout);
                team.shout_until = until;
                let boost = match shout {
                    Shout::Encourage => 1.02,
                    Shout::DemandMore => 1.035,
                    _ => 1.0,
                };
                for pl in &mut team.players {
                    pl.boost = boost;
                }
            }
        }
        self.applied.push(TimedInstruction {
            step: self.steps,
            side,
            instruction,
        });
        Ok(())
    }

    fn expire_shouts(&mut self) {
        let now = self.elapsed();
        for team in &mut self.teams {
            if team.shout.is_some() && now >= team.shout_until {
                team.shout = None;
                for pl in &mut team.players {
                    pl.boost = 1.0;
                }
            }
        }
    }

    /// Switches formation and sliders; players keep their slots.
    fn change_tactic(&mut self, side: usize, tactic: Tactic) {
        let team = &mut self.teams[side];
        team.tactic = tactic.clamped();
        team.slots = team.tactic.formation.slots();
        for s in 0..11 {
            if let Some(i) = team.on[s] {
                let position = team.slots[s].position;
                let pl = &mut team.players[i];
                pl.skills = Skills::of(&pl.snap, position);
                pl.defensive = is_defensive(position);
            }
        }
        team.rebuild_weights();
    }

    pub(super) fn minute(&self) -> u8 {
        let m = (self.t / 60.0) as u32 + 1;
        let cap = [45, 90, 105, 120][usize::from(self.half.min(3))];
        m.min(cap) as u8
    }

    pub(super) fn kickoff(&mut self, side: usize) {
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

    pub(super) fn half_time_recovery(&mut self) {
        for team in &mut self.teams {
            for p in &mut team.players {
                p.energy = (p.energy + 0.06).min(1.0);
            }
        }
    }

    pub(super) fn minute_tick(&mut self, at: f64) {
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
            let closing = team.markings(crate::roles::Marking::CloseDown) as f64;
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
                    if team.shout == Some(Shout::DemandMore) {
                        drain *= 1.25;
                    }
                    if closing > 0.0 {
                        drain *= 1.0 + self.p.close_drain * closing;
                    }
                    pl.energy = (pl.energy - drain).max(0.0);
                }
            }
            if !self.teams[side].auto_subs {
                continue;
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
    pub(super) fn ai_substitutions(&mut self, side: usize, max: u8, threshold: f64) {
        let (max_subs, max_windows) = (self.max_subs(), self.max_windows());
        let team = &self.teams[side];
        if team.windows_used >= max_windows || team.subs_made >= max_subs || team.bench.is_empty() {
            return;
        }
        let mut tired: Vec<(usize, f64)> = (1..11)
            .filter_map(|s| team.on[s].map(|i| (s, team.players[i].energy)))
            .filter(|&(_, e)| e < threshold)
            .collect();
        tired.sort_by(|a, b| a.1.total_cmp(&b.1).then(a.0.cmp(&b.0)));
        let mut made = 0;
        for (slot, _) in tired {
            if made == max || self.teams[side].subs_made >= max_subs {
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
    pub(super) fn substitute(&mut self, side: usize, slot: usize) -> bool {
        let team = &mut self.teams[side];
        let position = team.slots[slot].position;
        let Some(k) = (0..team.bench.len()).max_by(|&a, &b| {
            let sa = Skills::of(&team.players[team.bench[a]].snap, position).overall;
            let sb = Skills::of(&team.players[team.bench[b]].snap, position).overall;
            sa.total_cmp(&sb).then(b.cmp(&a))
        }) else {
            return false;
        };
        self.bring_on(side, slot, k);
        true
    }

    /// Replaces the player in `slot` with bench entry `k`.
    pub(super) fn bring_on(&mut self, side: usize, slot: usize, k: usize) {
        let minute = self.minute();
        let team = &mut self.teams[side];
        let position = team.slots[slot].position;
        let incoming = team.bench.remove(k);
        let outgoing = team.on[slot].expect("slot occupied");
        team.players[outgoing].slot = None;
        team.players[outgoing].off_minute = Some(minute);
        let pl = &mut team.players[incoming];
        pl.skills = Skills::of(&pl.snap, position);
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
    }

    pub(super) fn pick(
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
        // Marked players are harder to find.
        let opponent = &self.teams[1 - side];
        if attack && !opponent.instructions.marking.is_empty() {
            for (s, weight) in w.iter_mut().enumerate() {
                if let Some(i) = team.on[s] {
                    match opponent.marking(team.players[i].snap.id) {
                        Some(crate::roles::Marking::Tight) => *weight *= 0.75,
                        Some(crate::roles::Marking::CloseDown) => *weight *= 0.9,
                        None => {}
                    }
                }
            }
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

    pub(super) fn home_logit(&self, side: usize) -> f64 {
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
    pub(super) fn mentality(&self, side: usize) -> f64 {
        let base = f64::from(self.teams[side].tactic.mentality);
        let diff = match self.decider {
            Some(d) => d.aggregate_diff(self.goals, side),
            None => i32::from(self.goals[side]) - i32::from(self.goals[1 - side]),
        };
        let minute = self.t / 60.0;
        let shift = match diff {
            d if d < 0 && minute >= 80.0 => 2.0,
            d if d < 0 && minute >= 60.0 => 1.0,
            d if d >= 2 => -1.0,
            d if d > 0 && minute >= 70.0 => -1.0,
            _ => 0.0,
        };
        let shout = match self.teams[side].shout {
            Some(Shout::PushForward) => 1.0,
            Some(Shout::TightenUp) => -1.0,
            _ => 0.0,
        };
        if shout == 0.0 {
            (base + shift).clamp(-2.0, 2.0)
        } else {
            (base + shift + shout).clamp(-2.0, 2.0)
        }
    }

    /// Logit bonus for attacking near goal against `def`'s current
    /// mentality: attacking sides commit players forward and leave space
    /// behind; defensive sides crowd their own box.
    pub(super) fn exposure(&self, def: usize, band: usize) -> f64 {
        if band >= 3 {
            self.p.exposure_effect * self.mentality(def)
        } else {
            0.0
        }
    }

    /// Logit penalty from the defending team's pressing in this band.
    pub(super) fn pressure(&self, def: usize, band: usize) -> f64 {
        let press = f64::from(self.teams[def].tactic.pressing);
        if band <= 2 {
            self.p.press_effect * press
        } else if band >= 4 && press < 0.0 {
            -0.06 * press
        } else {
            0.0
        }
    }

    pub(super) fn turnover(
        &mut self,
        to_side: usize,
        band: usize,
        lane: usize,
        carrier: Option<usize>,
    ) {
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
    pub(super) fn finish(mut self) -> MatchResult {
        let total_play = (self.teams[0].in_play_secs + self.teams[1].in_play_secs).max(1.0);
        let extra_time = self.half >= 2;
        let full = if extra_time { 120 } else { 90 };
        let mut defensive = [Vec::new(), Vec::new()];
        let mut players = [Vec::new(), Vec::new()];
        let mut stats = [TeamStats::default(); 2];
        for side in 0..2 {
            let team = &mut self.teams[side];
            team.stats.possession = (team.in_play_secs / total_play) as f32;
            stats[side] = team.stats;
            for pl in &mut team.players {
                if pl.stats.started || pl.stats.minutes > 0 {
                    let off = pl.off_minute.unwrap_or(full);
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
            extra_time,
            shootout: self.shootout.take(),
        };
        rate_players(&mut result, &defensive);
        result
    }
}
