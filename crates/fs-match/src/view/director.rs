//! Stages tier 1 actions on a pitch.
//!
//! Each action becomes a ball movement between two points (with height
//! for lofted balls and shots). Players hold their formation shape,
//! shifted toward the ball and compacted when defending; whoever is
//! involved in the current action (passer, receiver, challenger) moves to
//! the ball. Everyone moves at running speed, so the picture flows rather
//! than jumping between zones.
//!
//! Staging is purely visual: it never affects the result, and it has its
//! own deterministic jitter so the same match always looks the same.

use super::frame::{FRAME_HZ, Frame, PITCH_LENGTH, PITCH_WIDTH, PlayerFrame, Pose};
use crate::formation::Slot;
use crate::log::{ActionKind, ActionRecord, Outcome, Zone};
use crate::result::Side;
use crate::tier1::LiveLineup;
use fs_core::ids::PlayerId;
use fs_core::math;
use fs_core::rng::mix;
use std::collections::BTreeMap;

const LANE_Y: [f32; 3] = [10.2, 34.0, 57.8];
const CENTRE: [f32; 2] = [PITCH_LENGTH / 2.0, PITCH_WIDTH / 2.0];
/// Metres per second for players involved in the action / everyone else.
const SPRINT: f32 = 7.5;
const JOG: f32 = 5.0;

#[derive(Debug, Clone, Copy)]
struct Body {
    x: f32,
    y: f32,
    facing: f32,
}

/// Turns action records into frames.
#[derive(Debug, Default)]
pub struct Director {
    actions: Vec<ActionRecord>,
    cursor: usize,
    bodies: BTreeMap<PlayerId, Body>,
    next_t: f64,
    /// Half of the last rendered frame; teams swap ends between halves.
    half: u8,
}

impl Director {
    /// A director for a match about to kick off.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds newly played actions (in order).
    pub fn push(&mut self, actions: &[ActionRecord]) {
        self.actions.extend_from_slice(actions);
    }

    /// Time of the next frame to be rendered.
    pub fn rendered_until(&self) -> f64 {
        self.next_t
    }

    /// Renders frames up to `until` seconds. `half_starts` are the times
    /// at which each half after the first began (from
    /// [`crate::tier1::LiveMatch::half_starts`]). The lineups are the
    /// current ones; the actions covering `until` must already have been
    /// pushed.
    pub fn render(
        &mut self,
        until: f64,
        lineups: [&LiveLineup; 2],
        half_starts: &[f64],
    ) -> Vec<Frame> {
        let step = 1.0 / f64::from(FRAME_HZ);
        let mut frames = Vec::new();
        while self.next_t <= until {
            let t = self.next_t;
            frames.push(self.frame_at(t, lineups, half_starts));
            self.next_t += step;
        }
        frames
    }

    fn frame_at(&mut self, t: f64, lineups: [&LiveLineup; 2], half_starts: &[f64]) -> Frame {
        let half = half_of(t, half_starts);
        while self.cursor + 1 < self.actions.len()
            && f64::from(self.actions[self.cursor + 1].at) <= t
        {
            self.cursor += 1;
        }
        let action = self
            .actions
            .get(self.cursor)
            .filter(|a| f64::from(a.at) <= t)
            .copied();
        // The action belongs to the other half (the gap at half time).
        let action = action.filter(|a| half_of(f64::from(a.at), half_starts) == half);
        let scene = Scene::new(action, t, half);
        let swap_ends = half != self.half;
        self.half = half;

        let mut players = Vec::with_capacity(22);
        for (s, lineup) in lineups.iter().enumerate() {
            let side = Side::from_index(s);
            let slots = lineup.tactic.formation.slots();
            for live in lineup.on_pitch.iter().flatten() {
                let slot = slots[usize::from(live.slot)];
                let (target, speed, pose) = scene.target_for(side, slot, live.id, live.slot == 0);
                let body = self.bodies.entry(live.id).or_insert_with(|| {
                    // New arrivals (substitutes) come on at the halfway line.
                    if t > 1.0 {
                        Body {
                            x: CENTRE[0],
                            y: PITCH_WIDTH,
                            facing: 0.0,
                        }
                    } else {
                        Body {
                            x: target[0],
                            y: target[1],
                            facing: 0.0,
                        }
                    }
                });
                let moved = if swap_ends {
                    body.x = target[0];
                    body.y = target[1];
                    0.0
                } else {
                    move_toward(body, target, speed / FRAME_HZ)
                };
                body.facing = math::f32::atan2(scene.ball[1] - body.y, scene.ball[0] - body.x);
                let pose = pose.unwrap_or(if moved > 0.25 {
                    Pose::Running
                } else {
                    Pose::Standing
                });
                players.push(PlayerFrame {
                    id: live.id,
                    side,
                    x: body.x,
                    y: body.y,
                    facing: body.facing,
                    pose,
                });
            }
        }
        Frame {
            t: t as f32,
            ball: scene.ball,
            players,
        }
    }
}

fn half_of(t: f64, half_starts: &[f64]) -> u8 {
    half_starts.iter().filter(|&&s| t >= s).count() as u8
}

/// Moves a body toward a target by at most `max` metres; returns the
/// distance moved.
fn move_toward(body: &mut Body, target: [f32; 2], max: f32) -> f32 {
    let dx = target[0] - body.x;
    let dy = target[1] - body.y;
    let d = math::f32::sqrt(dx * dx + dy * dy);
    if d <= max || d == 0.0 {
        body.x = target[0];
        body.y = target[1];
        return d;
    }
    body.x += dx / d * max;
    body.y += dy / d * max;
    max
}

/// +1 if `side` attacks toward x = 105 in this half, -1 otherwise.
fn direction(side: Side, half: u8) -> f32 {
    let home_right = half % 2 == 0;
    if (side == Side::Home) == home_right {
        1.0
    } else {
        -1.0
    }
}

/// Converts a point in `side`'s attacking frame (x along their attack,
/// y from their left touchline) to pitch coordinates.
fn to_pitch(side: Side, half: u8, xa: f32, ya: f32) -> [f32; 2] {
    if direction(side, half) > 0.0 {
        [xa, ya]
    } else {
        [PITCH_LENGTH - xa, PITCH_WIDTH - ya]
    }
}

/// Converts a pitch point into `side`'s attacking frame.
fn to_frame(side: Side, half: u8, p: [f32; 2]) -> [f32; 2] {
    if direction(side, half) > 0.0 {
        p
    } else {
        [PITCH_LENGTH - p[0], PITCH_WIDTH - p[1]]
    }
}

/// A deterministic offset in [-1, 1) from two keys.
fn wobble(a: u64, b: u64) -> f32 {
    (mix(a.wrapping_mul(0x9E37_79B9) ^ b) >> 40) as f32 / (1u64 << 23) as f32 - 1.0
}

fn zone_point(side: Side, half: u8, zone: Zone, key: u64) -> [f32; 2] {
    let xa = (f32::from(zone.band) + 0.5) / 6.0 * PITCH_LENGTH + 6.0 * wobble(key, 1);
    let ya = LANE_Y[usize::from(zone.lane.min(2))] + 5.0 * wobble(key, 2);
    to_pitch(
        side,
        half,
        xa.clamp(2.0, PITCH_LENGTH - 2.0),
        ya.clamp(2.0, PITCH_WIDTH - 2.0),
    )
}

/// Possession changes at the end of these outcomes.
fn turns_over(a: &ActionRecord) -> bool {
    match a.kind {
        ActionKind::Foul => true,
        _ => matches!(
            a.outcome,
            Outcome::Intercepted
                | Outcome::Tackled
                | Outcome::OutOfPlay
                | Outcome::Offside
                | Outcome::Saved
                | Outcome::Missed
                | Outcome::Blocked
                | Outcome::Goal
                | Outcome::Cleared
        ),
    }
}

/// What is happening at one instant, from which targets follow.
struct Scene {
    action: Option<ActionRecord>,
    half: u8,
    /// Seconds into the current action.
    dt: f32,
    ball: [f32; 3],
    /// Where the ball is going (end of the current action).
    ball_to: [f32; 2],
    /// Side in possession right now.
    possession: Option<Side>,
    /// Teams line up in their own halves (kickoffs).
    kickoff: bool,
    /// The side celebrating a goal, and where.
    celebrating: Option<(Side, [f32; 2])>,
}

impl Scene {
    fn new(action: Option<ActionRecord>, t: f64, half: u8) -> Self {
        let Some(a) = action else {
            return Self {
                action: None,
                half,
                dt: 0.0,
                ball: [CENTRE[0], CENTRE[1], 0.0],
                ball_to: CENTRE,
                possession: None,
                kickoff: true,
                celebrating: None,
            };
        };
        let dt = (t - f64::from(a.at)) as f32;
        let key = u64::from(a.step) << 8 | a.kind as u64;
        let attack = a.side;
        let mut p0 = zone_point(attack, half, a.from, key);
        let goal = to_pitch(attack, half, PITCH_LENGTH, PITCH_WIDTH / 2.0);
        match a.kind {
            ActionKind::Penalty => {
                p0 = to_pitch(attack, half, PITCH_LENGTH - 11.0, PITCH_WIDTH / 2.0)
            }
            ActionKind::Corner => {
                let ya = if a.from.lane == 0 {
                    0.5
                } else {
                    PITCH_WIDTH - 0.5
                };
                p0 = to_pitch(attack, half, PITCH_LENGTH - 0.5, ya);
            }
            _ => {}
        }
        let shot = matches!(
            a.kind,
            ActionKind::Shot | ActionKind::Header | ActionKind::Penalty | ActionKind::FreeKick
        );
        let p1 = if shot {
            let spread = match a.outcome {
                Outcome::Goal => 3.0 * wobble(key, 3),
                Outcome::Saved => 2.0 * wobble(key, 3),
                Outcome::Missed => (5.0 + 3.0 * wobble(key, 4).abs()) * wobble(key, 3).signum(),
                _ => 0.0,
            };
            let on_line = to_pitch(attack, half, PITCH_LENGTH, PITCH_WIDTH / 2.0 + spread);
            if a.outcome == Outcome::Blocked {
                [
                    p0[0] + 0.4 * (on_line[0] - p0[0]),
                    p0[1] + 0.4 * (on_line[1] - p0[1]),
                ]
            } else {
                on_line
            }
        } else if a.kind == ActionKind::Foul {
            p0
        } else {
            zone_point(attack, half, a.to, key ^ 0xABCD)
        };
        let height = match a.kind {
            ActionKind::LongPass => 9.0,
            ActionKind::Corner => 7.0,
            ActionKind::Cross => 6.0,
            ActionKind::FreeKick => 2.5,
            ActionKind::Shot if a.outcome == Outcome::Missed => 3.0,
            ActionKind::Shot | ActionKind::Penalty => 1.0,
            ActionKind::ThroughBall => 0.5,
            _ => 0.2,
        };
        let in_play = dt < a.duration;
        let ball = if in_play {
            let p = (dt / a.duration.max(0.01)).clamp(0.0, 1.0);
            let z = if a.kind == ActionKind::Header {
                2.3 - 1.8 * p
            } else {
                4.0 * height * p * (1.0 - p)
            };
            [p0[0] + (p1[0] - p0[0]) * p, p0[1] + (p1[1] - p0[1]) * p, z]
        } else if a.outcome == Outcome::Goal {
            // In the net, then back to the centre spot for the restart.
            if dt - a.duration < a.dead * 0.7 {
                let beyond = to_pitch(
                    attack,
                    half,
                    PITCH_LENGTH + 1.2,
                    to_frame(attack, half, p1)[1],
                );
                [beyond[0], beyond[1], 0.0]
            } else {
                [CENTRE[0], CENTRE[1], 0.0]
            }
        } else {
            [p1[0], p1[1], 0.0]
        };
        let possession = if in_play || !turns_over(&a) {
            Some(attack)
        } else {
            Some(attack.other())
        };
        let celebrating =
            (a.outcome == Outcome::Goal && !in_play && dt - a.duration < a.dead * 0.7).then(|| {
                (
                    attack,
                    [
                        goal[0] - 12.0 * direction(attack, half),
                        goal[1] + 10.0 * wobble(key, 5),
                    ],
                )
            });
        let kickoff = a.outcome == Outcome::Goal && !in_play && dt - a.duration >= a.dead * 0.7;
        Self {
            action: Some(a),
            half,
            dt,
            ball,
            ball_to: p1,
            possession,
            kickoff,
            celebrating,
        }
    }

    /// Where a player wants to be, how fast they go, and any special pose.
    fn target_for(
        &self,
        side: Side,
        slot: Slot,
        id: PlayerId,
        keeper: bool,
    ) -> ([f32; 2], f32, Option<Pose>) {
        let half = self.half;
        let ball_frame = to_frame(side, half, [self.ball[0], self.ball[1]]);
        let bx = ball_frame[0] / PITCH_LENGTH;
        let by = ball_frame[1] / PITCH_WIDTH;
        let seed = u64::from(id.raw());
        let (jx, jy) = (2.0 * wobble(seed, 7), 2.0 * wobble(seed, 8));

        if let Some((celebrating, spot)) = self.celebrating
            && celebrating == side
            && !keeper
        {
            return (
                [spot[0] + jx * 2.0, spot[1] + jy * 2.0],
                SPRINT,
                Some(Pose::Celebrating),
            );
        }

        // Shape: formation slot shifted with the ball.
        let (fx, fy) = (slot.x as f32, slot.y as f32);
        let anchor_frame = if keeper {
            [
                2.5 + 8.0 * bx,
                PITCH_WIDTH / 2.0 + (ball_frame[1] - PITCH_WIDTH / 2.0) * 0.15,
            ]
        } else if self.kickoff {
            [
                fx * 0.46 * PITCH_LENGTH + jx,
                (0.5 + (fy - 0.5) * 0.9) * PITCH_WIDTH + jy,
            ]
        } else {
            let with_ball = self.possession == Some(side);
            let ax = if with_ball {
                fx * 0.72 + 0.28 * bx + 0.02
            } else {
                fx * 0.55 + 0.30 * bx + 0.02
            };
            let ay = 0.5 + (fy - 0.5) * 0.85 + (by - 0.5) * 0.25;
            [ax * PITCH_LENGTH + jx, ay * PITCH_WIDTH + jy]
        };
        let anchor = to_pitch(
            side,
            half,
            anchor_frame[0].clamp(1.0, PITCH_LENGTH - 1.0),
            anchor_frame[1].clamp(1.0, PITCH_WIDTH - 1.0),
        );

        let Some(a) = self.action else {
            return (anchor, JOG, None);
        };
        let in_play = self.dt < a.duration;
        let early = self.dt < a.duration * 0.35;
        let ball = [self.ball[0], self.ball[1]];
        let pose_for_kind = match a.kind {
            ActionKind::Shot | ActionKind::Penalty | ActionKind::FreeKick => Pose::Shooting,
            ActionKind::Header => Pose::Heading,
            ActionKind::Foul => Pose::Tackling,
            ActionKind::Dribble => Pose::Running,
            _ => Pose::Passing,
        };
        if id == a.actor && side == a.side && in_play {
            if early {
                return (ball, SPRINT, Some(pose_for_kind));
            }
            if a.kind == ActionKind::Dribble {
                return (ball, SPRINT, None);
            }
        }
        if Some(id) == a.target && side == a.side && in_play {
            return (self.ball_to, SPRINT, None);
        }
        if Some(id) == a.opponent && side != a.side && in_play {
            if keeper && a.outcome == Outcome::Saved && self.dt > a.duration * 0.6 {
                return (self.ball_to, SPRINT, Some(Pose::Diving));
            }
            let pose = (a.outcome == Outcome::Tackled && !early).then_some(Pose::Tackling);
            return (ball, SPRINT, pose);
        }
        (anchor, JOG, None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::input;
    use crate::tier1::LiveMatch;

    fn watch(key: u64) -> Vec<Frame> {
        let i = input(600, 600, key);
        let mut live = LiveMatch::new(&i);
        let mut d = Director::new();
        let mut frames = Vec::new();
        let mut t = 0.0;
        while !live.is_finished() {
            t += 30.0;
            live.advance_until(t + 10.0);
            d.push(&live.take_actions());
            let starts = live.half_starts().to_vec();
            let lineups = [live.lineup(Side::Home), live.lineup(Side::Away)];
            frames.extend(d.render(t.min(live.elapsed()), [&lineups[0], &lineups[1]], &starts));
        }
        frames
    }

    #[test]
    fn frames_cover_the_match_and_stay_on_the_pitch() {
        let frames = watch(1);
        assert!(
            frames.len() as f32 > 90.0 * 60.0 * FRAME_HZ * 0.98,
            "{}",
            frames.len()
        );
        for f in &frames {
            assert!(
                f.players.len() >= 20 && f.players.len() <= 22,
                "{} players",
                f.players.len()
            );
            let [x, y, z] = f.ball;
            assert!(
                (-3.0..=PITCH_LENGTH + 3.0).contains(&x) && (-1.0..=PITCH_WIDTH + 1.0).contains(&y),
                "ball {x} {y}"
            );
            assert!((0.0..=12.0).contains(&z), "ball height {z}");
            for p in &f.players {
                assert!(
                    (0.0..=PITCH_LENGTH).contains(&p.x) && (0.0..=PITCH_WIDTH).contains(&p.y),
                    "{p:?}"
                );
            }
        }
        // Frames are evenly spaced.
        assert!(
            frames
                .windows(2)
                .all(|w| (w[1].t - w[0].t - 1.0 / FRAME_HZ).abs() < 1e-3)
        );
    }

    #[test]
    fn players_move_at_human_speeds() {
        let frames = watch(2);
        let mut last: BTreeMap<PlayerId, (f32, f32)> = BTreeMap::new();
        let mut fast = 0;
        let mut total = 0;
        for f in &frames {
            for p in &f.players {
                if let Some(&(x, y)) = last.get(&p.id) {
                    let (dx, dy) = (p.x - x, p.y - y);
                    let d = (dx * dx + dy * dy).sqrt();
                    total += 1;
                    if d > SPRINT / FRAME_HZ + 0.01 {
                        fast += 1;
                    }
                }
                last.insert(p.id, (p.x, p.y));
            }
        }
        // Only the half-time swap of ends teleports players.
        assert!(
            fast <= 24,
            "{fast} of {total} moves were faster than a sprint"
        );
    }

    #[test]
    fn same_match_looks_the_same() {
        assert_eq!(watch(3), watch(3));
    }
}
