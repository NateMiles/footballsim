//! Roles, set-piece takers and opposition instructions.

use super::refs::*;
use crate::dto;
use fs_match::roles::{Duty, Effect, Instructions, MAX_MARKING, Marking, Role, SlotRole};
use fs_world::{PlayerId, World};

impl From<Role> for dto::PlayerRole {
    fn from(r: Role) -> Self {
        match r {
            Role::Goalkeeper => dto::PlayerRole::Goalkeeper,
            Role::SweeperKeeper => dto::PlayerRole::SweeperKeeper,
            Role::CentralDefender => dto::PlayerRole::CentralDefender,
            Role::BallPlayingDefender => dto::PlayerRole::BallPlayingDefender,
            Role::FullBack => dto::PlayerRole::FullBack,
            Role::WingBack => dto::PlayerRole::WingBack,
            Role::DefensiveMidfielder => dto::PlayerRole::DefensiveMidfielder,
            Role::BallWinner => dto::PlayerRole::BallWinner,
            Role::DeepPlaymaker => dto::PlayerRole::DeepPlaymaker,
            Role::CentralMidfielder => dto::PlayerRole::CentralMidfielder,
            Role::BoxToBox => dto::PlayerRole::BoxToBox,
            Role::Playmaker => dto::PlayerRole::Playmaker,
            Role::WideMidfielder => dto::PlayerRole::WideMidfielder,
            Role::Winger => dto::PlayerRole::Winger,
            Role::InsideForward => dto::PlayerRole::InsideForward,
            Role::WideForward => dto::PlayerRole::WideForward,
            Role::AttackingMidfielder => dto::PlayerRole::AttackingMidfielder,
            Role::ShadowStriker => dto::PlayerRole::ShadowStriker,
            Role::Striker => dto::PlayerRole::Striker,
            Role::TargetMan => dto::PlayerRole::TargetMan,
            Role::Poacher => dto::PlayerRole::Poacher,
            Role::FalseNine => dto::PlayerRole::FalseNine,
        }
    }
}

impl From<dto::PlayerRole> for Role {
    fn from(r: dto::PlayerRole) -> Self {
        match r {
            dto::PlayerRole::Goalkeeper => Role::Goalkeeper,
            dto::PlayerRole::SweeperKeeper => Role::SweeperKeeper,
            dto::PlayerRole::CentralDefender => Role::CentralDefender,
            dto::PlayerRole::BallPlayingDefender => Role::BallPlayingDefender,
            dto::PlayerRole::FullBack => Role::FullBack,
            dto::PlayerRole::WingBack => Role::WingBack,
            dto::PlayerRole::DefensiveMidfielder => Role::DefensiveMidfielder,
            dto::PlayerRole::BallWinner => Role::BallWinner,
            dto::PlayerRole::DeepPlaymaker => Role::DeepPlaymaker,
            dto::PlayerRole::CentralMidfielder => Role::CentralMidfielder,
            dto::PlayerRole::BoxToBox => Role::BoxToBox,
            dto::PlayerRole::Playmaker => Role::Playmaker,
            dto::PlayerRole::WideMidfielder => Role::WideMidfielder,
            dto::PlayerRole::Winger => Role::Winger,
            dto::PlayerRole::InsideForward => Role::InsideForward,
            dto::PlayerRole::WideForward => Role::WideForward,
            dto::PlayerRole::AttackingMidfielder => Role::AttackingMidfielder,
            dto::PlayerRole::ShadowStriker => Role::ShadowStriker,
            dto::PlayerRole::Striker => Role::Striker,
            dto::PlayerRole::TargetMan => Role::TargetMan,
            dto::PlayerRole::Poacher => Role::Poacher,
            dto::PlayerRole::FalseNine => Role::FalseNine,
        }
    }
}

impl From<Duty> for dto::Duty {
    fn from(d: Duty) -> Self {
        match d {
            Duty::Defend => dto::Duty::Defend,
            Duty::Support => dto::Duty::Support,
            Duty::Attack => dto::Duty::Attack,
        }
    }
}

impl From<dto::Duty> for Duty {
    fn from(d: dto::Duty) -> Self {
        match d {
            dto::Duty::Defend => Duty::Defend,
            dto::Duty::Support => Duty::Support,
            dto::Duty::Attack => Duty::Attack,
        }
    }
}

impl From<Marking> for dto::Marking {
    fn from(m: Marking) -> Self {
        match m {
            Marking::Tight => dto::Marking::Tight,
            Marking::CloseDown => dto::Marking::CloseDown,
        }
    }
}

impl From<dto::Marking> for Marking {
    fn from(m: dto::Marking) -> Self {
        match m {
            dto::Marking::Tight => Marking::Tight,
            dto::Marking::CloseDown => Marking::CloseDown,
        }
    }
}

fn role_info(r: Role) -> dto::RoleInfo {
    dto::RoleInfo {
        role: r.into(),
        name: r.name().into(),
        description: r.description().into(),
    }
}

/// The instructions a club's team sheet carries now.
pub fn view(world: &World, sheet: &fs_match::TeamSheet) -> dto::InstructionsView {
    let ins = &sheet.instructions;
    let raw = |p: Option<PlayerId>| p.map(|p| p.raw());
    dto::InstructionsView {
        roles: sheet
            .tactic
            .formation
            .slots()
            .iter()
            .zip(&ins.roles)
            .map(|(s, r)| {
                let r = r.unwrap_or(SlotRole {
                    role: Role::standard(s.position),
                    duty: Duty::Support,
                });
                dto::SlotRoleView {
                    position: s.position.code().into(),
                    role: r.role.into(),
                    duty: r.duty.into(),
                    options: Role::options(s.position).iter().map(|&o| role_info(o)).collect(),
                    has_duty: s.position != fs_core::attributes::Position::Goalkeeper,
                }
            })
            .collect(),
        penalty_taker: raw(ins.penalty_taker),
        free_kick_taker: raw(ins.free_kick_taker),
        corner_taker: raw(ins.corner_taker),
        marking: ins
            .marking
            .iter()
            .filter(|(p, _)| world.players().contains(*p))
            .map(|&(p, m)| dto::MarkingView {
                player: player_ref(world, p),
                club: world.players().club(p).map(|c| club_ref(world, c)),
                marking: m.into(),
            })
            .collect(),
        markings: Marking::ALL
            .into_iter()
            .map(|m| dto::MarkingInfo {
                marking: m.into(),
                name: m.name().into(),
                description: match m {
                    Marking::Tight => {
                        "Harder for them to get and keep the ball, but the shape stretches and fouls come easier."
                    }
                    Marking::CloseDown => "Hurry them on the ball; the team tires faster.",
                }
                .into(),
            })
            .collect(),
        max_marking: MAX_MARKING as u8,
    }
}

/// Instructions from what the UI sent. Roles left at the slot's standard
/// role and support duty are stored as standard.
pub fn from_choices(
    formation: fs_match::Formation,
    roles: &[dto::RoleChoice],
    takers: [Option<PlayerId>; 3],
    marking: &[dto::MarkingChoice],
) -> Instructions {
    let mut ins = Instructions::default();
    for ((slot, choice), out) in formation
        .slots()
        .iter()
        .zip(roles)
        .zip(ins.roles.iter_mut())
    {
        let r = SlotRole {
            role: choice.role.into(),
            duty: choice.duty.into(),
        };
        if !Effect::of(slot.position, Some(r)).is_neutral() {
            *out = Some(r);
        }
    }
    [ins.penalty_taker, ins.free_kick_taker, ins.corner_taker] = takers;
    ins.marking = marking
        .iter()
        .map(|m| (PlayerId::new(m.player), m.marking.into()))
        .collect();
    ins
}
