//! Player roles and duties, set-piece takers and opposition instructions:
//! the instructions a manager gives beyond the team's shape and sliders.
//!
//! Every formation slot has a standard role that changes nothing, so a
//! team without instructions plays exactly as before. Other roles shift
//! what the player does on the ball (an [`Effect`]'s action weights) and
//! where they turn up (forward or back, inside, in the box, how much they
//! defend). The engine turns those shifts into a team's presence in each
//! zone and its defensive coverage, so pushing players on gains chances
//! and costs cover. Effects are kept small enough that no single choice is
//! worth more than a few points a season; the calibration report checks.

use crate::formation::Formation;
use fs_core::attributes::Position;
use fs_core::ids::PlayerId;
use serde::{Deserialize, Serialize};

/// How a player balances attack and defence.
///
/// The variant order is part of the save format. Only append.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum Duty {
    Defend,
    #[default]
    Support,
    Attack,
}

impl Duty {
    pub const ALL: [Duty; 3] = [Duty::Defend, Duty::Support, Duty::Attack];

    pub const fn name(self) -> &'static str {
        match self {
            Duty::Defend => "Defend",
            Duty::Support => "Support",
            Duty::Attack => "Attack",
        }
    }
}

/// What a player is asked to do in their slot.
///
/// The variant order is part of the save format. Only append.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Role {
    Goalkeeper,
    SweeperKeeper,
    CentralDefender,
    BallPlayingDefender,
    FullBack,
    WingBack,
    DefensiveMidfielder,
    BallWinner,
    DeepPlaymaker,
    CentralMidfielder,
    BoxToBox,
    Playmaker,
    WideMidfielder,
    Winger,
    InsideForward,
    WideForward,
    AttackingMidfielder,
    ShadowStriker,
    Striker,
    TargetMan,
    Poacher,
    FalseNine,
}

use Role::*;

impl Role {
    /// The role that changes nothing for a slot of this position.
    pub const fn standard(position: Position) -> Role {
        match position {
            Position::Goalkeeper => Goalkeeper,
            Position::DefenderCentre => CentralDefender,
            Position::DefenderLeft | Position::DefenderRight => FullBack,
            Position::WingBackLeft | Position::WingBackRight => WingBack,
            Position::DefensiveMidfielder => DefensiveMidfielder,
            Position::MidfielderCentre => CentralMidfielder,
            Position::MidfielderLeft | Position::MidfielderRight => WideMidfielder,
            Position::AttackingMidLeft | Position::AttackingMidRight => WideForward,
            Position::AttackingMidCentre => AttackingMidfielder,
            Position::Striker => Striker,
        }
    }

    /// The roles a slot of this position can take, standard first.
    pub const fn options(position: Position) -> &'static [Role] {
        match position {
            Position::Goalkeeper => &[Goalkeeper, SweeperKeeper],
            Position::DefenderCentre => &[CentralDefender, BallPlayingDefender],
            Position::DefenderLeft | Position::DefenderRight => &[FullBack, WingBack],
            Position::WingBackLeft | Position::WingBackRight => &[WingBack, FullBack],
            Position::DefensiveMidfielder => &[DefensiveMidfielder, BallWinner, DeepPlaymaker],
            Position::MidfielderCentre => &[CentralMidfielder, BoxToBox, Playmaker, BallWinner],
            Position::MidfielderLeft | Position::MidfielderRight => {
                &[WideMidfielder, Winger, InsideForward]
            }
            Position::AttackingMidLeft | Position::AttackingMidRight => {
                &[WideForward, Winger, InsideForward]
            }
            Position::AttackingMidCentre => &[AttackingMidfielder, Playmaker, ShadowStriker],
            Position::Striker => &[Striker, TargetMan, Poacher, FalseNine],
        }
    }

    pub const fn name(self) -> &'static str {
        match self {
            Goalkeeper => "Goalkeeper",
            SweeperKeeper => "Sweeper keeper",
            CentralDefender => "Central defender",
            BallPlayingDefender => "Ball-playing defender",
            FullBack => "Full-back",
            WingBack => "Wing-back",
            DefensiveMidfielder => "Defensive midfielder",
            BallWinner => "Ball winner",
            DeepPlaymaker => "Deep-lying playmaker",
            CentralMidfielder => "Central midfielder",
            BoxToBox => "Box-to-box midfielder",
            Playmaker => "Playmaker",
            WideMidfielder => "Wide midfielder",
            Winger => "Winger",
            InsideForward => "Inside forward",
            WideForward => "Wide forward",
            AttackingMidfielder => "Attacking midfielder",
            ShadowStriker => "Shadow striker",
            Striker => "Striker",
            TargetMan => "Target man",
            Poacher => "Poacher",
            FalseNine => "False nine",
        }
    }

    pub const fn description(self) -> &'static str {
        match self {
            Goalkeeper => "Keeps goal and distributes safely.",
            SweeperKeeper => "Plays short from the back instead of clearing long.",
            CentralDefender => "Defends first.",
            BallPlayingDefender => "Steps out with the ball and looks for longer passes.",
            FullBack => "Defends the flank and supports when it is safe.",
            WingBack => "Gets forward down the flank to cross, at some cost in defence.",
            DefensiveMidfielder => "Screens the defence.",
            BallWinner => "Hunts the ball and defends more; keeps it simple on the ball.",
            DeepPlaymaker => "Dictates play from deep with longer and through passes.",
            CentralMidfielder => "Links defence and attack.",
            BoxToBox => "Covers ground at both ends and arrives late in the box.",
            Playmaker => "Looks for the killer pass; does less without the ball.",
            WideMidfielder => "Works the flank at both ends.",
            Winger => "Takes on the full-back and crosses.",
            InsideForward => "Cuts inside to shoot instead of crossing.",
            WideForward => "Attacks down the flank.",
            AttackingMidfielder => "Plays between the lines.",
            ShadowStriker => "Runs beyond the striker to score; defends little.",
            Striker => "Leads the line.",
            TargetMan => "Holds the ball up and wins it in the air.",
            Poacher => "Lives in the box to finish chances; little else.",
            FalseNine => "Drops deep to create, leaving space for runners.",
        }
    }

    /// Whether the role suits a slot of this position.
    pub fn fits(self, position: Position) -> bool {
        Role::options(position).contains(&self)
    }
}

/// A slot's role and duty.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SlotRole {
    pub role: Role,
    pub duty: Duty,
}

/// What a role and duty change for one slot. The neutral effect changes
/// nothing.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Effect {
    /// Multipliers on the carrier's action weights: short pass, long ball,
    /// through ball, cross, dribble, shot.
    pub actions: [f64; 6],
    /// Shift of the player's attacking position (share of the pitch).
    pub forward: f64,
    /// Multiplier on defensive involvement.
    pub defending: f64,
    /// Pull toward the centre in the final third, 0-1.
    pub inside: f64,
    /// Multiplier on involvement in the penalty area.
    pub in_box: f64,
}

impl Effect {
    pub const NEUTRAL: Effect = Effect {
        actions: [1.0; 6],
        forward: 0.0,
        defending: 1.0,
        inside: 0.0,
        in_box: 1.0,
    };

    pub fn is_neutral(&self) -> bool {
        *self == Effect::NEUTRAL
    }

    /// The effect of a role and duty in a slot of `position`.
    pub fn of(position: Position, slot: Option<SlotRole>) -> Effect {
        let Some(SlotRole { role, duty }) = slot else {
            return Effect::NEUTRAL;
        };
        let mut e = Effect::NEUTRAL;
        // [short, long, through, cross, dribble, shot]
        let mut act = |m: [f64; 6]| {
            for (a, b) in e.actions.iter_mut().zip(m) {
                *a *= b;
            }
        };
        if role != Role::standard(position) {
            match role {
                SweeperKeeper => act([1.25, 0.6, 1.0, 1.0, 1.0, 1.0]),
                BallPlayingDefender => act([0.97, 1.25, 1.6, 1.0, 1.2, 1.0]),
                WingBack => {
                    act([1.0, 1.0, 1.0, 1.25, 1.1, 1.0]);
                    e.forward += 0.06;
                    e.defending *= 0.88;
                }
                FullBack => {
                    act([1.0, 1.0, 1.0, 0.8, 0.9, 1.0]);
                    e.forward -= 0.06;
                    e.defending *= 1.12;
                }
                BallWinner => {
                    act([1.0, 1.0, 0.7, 1.0, 0.8, 0.8]);
                    e.forward -= 0.02;
                    e.defending *= 1.2;
                }
                DeepPlaymaker => {
                    act([0.95, 1.3, 1.5, 1.0, 0.85, 1.0]);
                    e.defending *= 0.92;
                }
                BoxToBox => {
                    act([1.0, 1.0, 1.0, 1.0, 1.1, 1.2]);
                    e.forward += 0.04;
                }
                Playmaker => {
                    act([1.05, 1.0, 1.5, 1.0, 0.9, 0.85]);
                    e.defending *= 0.9;
                }
                Winger => {
                    act([0.92, 1.0, 1.0, 1.35, 1.2, 0.8]);
                    e.forward += 0.03;
                    e.defending *= 0.9;
                }
                InsideForward => {
                    act([1.0, 1.0, 1.0, 0.5, 1.2, 1.35]);
                    e.forward += 0.03;
                    e.inside = 0.35;
                    e.defending *= 0.88;
                }
                ShadowStriker => {
                    act([0.92, 1.0, 0.85, 1.0, 1.0, 1.35]);
                    e.forward += 0.06;
                    e.defending *= 0.88;
                }
                TargetMan => {
                    act([1.12, 1.0, 1.0, 1.0, 0.7, 1.0]);
                    e.forward -= 0.02;
                    e.in_box *= 1.25;
                }
                Poacher => {
                    act([0.88, 1.0, 0.8, 1.0, 0.85, 1.25]);
                    e.forward += 0.03;
                    e.in_box *= 1.3;
                    e.defending *= 0.7;
                }
                FalseNine => {
                    act([1.1, 1.0, 1.4, 1.0, 1.0, 0.85]);
                    e.forward -= 0.09;
                    e.in_box *= 0.8;
                    e.defending *= 1.05;
                }
                // Standard roles of other positions played here change
                // nothing beyond the duty.
                _ => {}
            }
        }
        if position != Position::Goalkeeper {
            match duty {
                Duty::Defend => {
                    act([1.0, 1.0, 1.0, 0.95, 0.95, 0.92]);
                    e.forward -= 0.05;
                    e.defending *= 1.1;
                }
                Duty::Support => {}
                Duty::Attack => {
                    act([1.0, 1.0, 1.0, 1.05, 1.05, 1.08]);
                    e.forward += 0.05;
                    e.defending *= 0.9;
                }
            }
        }
        e
    }
}

/// An instruction for dealing with one opposition player.
///
/// The variant order is part of the save format. Only append.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Marking {
    /// Stay tight: harder for them on the ball, but the shape stretches
    /// and fouls come more easily.
    Tight,
    /// Close them down at once: hurries them, but tires the team.
    CloseDown,
}

impl Marking {
    pub const ALL: [Marking; 2] = [Marking::Tight, Marking::CloseDown];

    pub const fn name(self) -> &'static str {
        match self {
            Marking::Tight => "Mark tightly",
            Marking::CloseDown => "Close down",
        }
    }
}

/// At most this many opposition instructions per match.
pub const MAX_MARKING: usize = 3;

/// A team's instructions beyond its tactic. The default changes nothing.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Instructions {
    /// Role and duty by formation slot; `None` keeps the standard role.
    pub roles: [Option<SlotRole>; 11],
    pub penalty_taker: Option<PlayerId>,
    pub free_kick_taker: Option<PlayerId>,
    pub corner_taker: Option<PlayerId>,
    /// Opposition instructions (at most [`MAX_MARKING`]).
    pub marking: Vec<(PlayerId, Marking)>,
}

impl Instructions {
    /// Checks the roles fit the formation's slots.
    ///
    /// # Errors
    /// Describes the first problem.
    pub fn validate(&self, formation: Formation) -> Result<(), String> {
        for (slot, r) in formation.slots().iter().zip(&self.roles) {
            if let Some(r) = r
                && !r.role.fits(slot.position)
            {
                return Err(format!(
                    "a {} cannot play {}",
                    r.role.name(),
                    slot.position.code()
                ));
            }
        }
        if self.marking.len() > MAX_MARKING {
            return Err(format!("at most {MAX_MARKING} opposition instructions"));
        }
        let mut ids: Vec<PlayerId> = self.marking.iter().map(|m| m.0).collect();
        ids.sort_unstable();
        if ids.windows(2).any(|w| w[0] == w[1]) {
            return Err("one instruction per opposition player".into());
        }
        Ok(())
    }

    /// The same instructions without anything that applies to one match.
    pub fn standing(&self) -> Instructions {
        Instructions {
            marking: Vec::new(),
            ..self.clone()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn standard_roles_change_nothing() {
        for f in Formation::ALL {
            for s in f.slots() {
                let r = Role::standard(s.position);
                assert_eq!(Role::options(s.position)[0], r);
                let e = Effect::of(
                    s.position,
                    Some(SlotRole {
                        role: r,
                        duty: Duty::Support,
                    }),
                );
                assert!(e.is_neutral(), "{r:?}");
            }
        }
        assert!(Effect::of(Position::Striker, None).is_neutral());
    }

    #[test]
    fn roles_must_fit_the_slot() {
        let mut i = Instructions::default();
        i.roles[0] = Some(SlotRole {
            role: Poacher,
            duty: Duty::Attack,
        });
        assert!(i.validate(Formation::F442).is_err());
        i.roles[0] = Some(SlotRole {
            role: SweeperKeeper,
            duty: Duty::Support,
        });
        assert_eq!(i.validate(Formation::F442), Ok(()));
    }
}
