//! The training screen.

use super::refs::*;
use crate::dto;
use fs_world::attributes::{Attr, Position};
use fs_world::training::{Focus, Individual, Intensity, Schedule};
use fs_world::{ClubId, World};

impl From<Focus> for dto::TrainingFocus {
    fn from(f: Focus) -> Self {
        match f {
            Focus::Balanced => dto::TrainingFocus::Balanced,
            Focus::Fitness => dto::TrainingFocus::Fitness,
            Focus::Attacking => dto::TrainingFocus::Attacking,
            Focus::Defending => dto::TrainingFocus::Defending,
            Focus::Tactics => dto::TrainingFocus::Tactics,
            Focus::Recovery => dto::TrainingFocus::Recovery,
        }
    }
}

impl From<dto::TrainingFocus> for Focus {
    fn from(f: dto::TrainingFocus) -> Self {
        match f {
            dto::TrainingFocus::Balanced => Focus::Balanced,
            dto::TrainingFocus::Fitness => Focus::Fitness,
            dto::TrainingFocus::Attacking => Focus::Attacking,
            dto::TrainingFocus::Defending => Focus::Defending,
            dto::TrainingFocus::Tactics => Focus::Tactics,
            dto::TrainingFocus::Recovery => Focus::Recovery,
        }
    }
}

impl From<Intensity> for dto::TrainingIntensity {
    fn from(i: Intensity) -> Self {
        match i {
            Intensity::Light => dto::TrainingIntensity::Light,
            Intensity::Normal => dto::TrainingIntensity::Normal,
            Intensity::Hard => dto::TrainingIntensity::Hard,
        }
    }
}

impl From<dto::TrainingIntensity> for Intensity {
    fn from(i: dto::TrainingIntensity) -> Self {
        match i {
            dto::TrainingIntensity::Light => Intensity::Light,
            dto::TrainingIntensity::Normal => Intensity::Normal,
            dto::TrainingIntensity::Hard => Intensity::Hard,
        }
    }
}

/// Individual training as the UI names it.
pub fn individual_dto(t: Individual) -> dto::IndividualTraining {
    match t {
        Individual::Position(p) => dto::IndividualTraining::Position {
            position: p.code().to_owned(),
        },
        Individual::Attribute(a) => dto::IndividualTraining::Attribute {
            attribute: a.label().to_owned(),
        },
    }
}

/// Individual training from the UI's names, if they are known.
pub fn individual_from(t: &dto::IndividualTraining) -> Option<Individual> {
    match t {
        dto::IndividualTraining::Position { position } => Position::ALL
            .into_iter()
            .find(|p| p.code() == position)
            .map(Individual::Position),
        dto::IndividualTraining::Attribute { attribute } => Attr::ALL
            .into_iter()
            .find(|a| a.label() == attribute)
            .map(Individual::Attribute),
    }
}

fn focus_info(f: Focus) -> dto::FocusInfo {
    let (name, description) = match f {
        Focus::Balanced => ("Balanced", "A bit of everything."),
        Focus::Fitness => (
            "Fitness",
            "Pace, stamina and strength grow faster; recovery a little quicker.",
        ),
        Focus::Attacking => (
            "Attacking",
            "Finishing, dribbling, passing and movement grow faster, at the expense of the rest.",
        ),
        Focus::Defending => (
            "Defending",
            "Tackling, marking, positioning and heading grow faster, at the expense of the rest.",
        ),
        Focus::Tactics => (
            "Tactics",
            "Decision making and teamwork grow faster, and formations are learnt twice as fast.",
        ),
        Focus::Recovery => (
            "Recovery",
            "Rest: players recover faster and pick up fewer knocks, but grow more slowly.",
        ),
    };
    dto::FocusInfo {
        focus: f.into(),
        name: name.into(),
        description: description.into(),
    }
}

fn intensity_info(i: Intensity) -> dto::IntensityInfo {
    let (name, description) = match i {
        Intensity::Light => (
            "Light",
            "Slower growth, quicker recovery, fewer training injuries.",
        ),
        Intensity::Normal => ("Normal", "The usual load."),
        Intensity::Hard => (
            "Hard",
            "Faster growth, but players tire and get injured more.",
        ),
    };
    dto::IntensityInfo {
        intensity: i.into(),
        name: name.into(),
        description: description.into(),
    }
}

/// The schedule as the UI sends it.
pub fn schedule(focus: dto::TrainingFocus, intensity: dto::TrainingIntensity) -> Schedule {
    Schedule {
        focus: focus.into(),
        intensity: intensity.into(),
    }
}

pub fn view(world: &World, club: ClubId) -> dto::TrainingView {
    let c = &world.clubs()[club];
    let s = c.training();
    let pct = |x: f64| ((x - 1.0) * 100.0).round() as i16;
    let current = c.tactic().map(|t| t.formation);
    let ps = world.players();
    let today = world.date();
    let mut players: Vec<dto::TrainingPlayer> = c
        .squad()
        .iter()
        .map(|&p| {
            let own = c.individual_training().get(&p).copied();
            dto::TrainingPlayer {
                player: player_ref(world, p),
                age: age(world, p),
                individual: own.map(individual_dto),
                progress: match own {
                    Some(Individual::Position(pos)) => Some(ps.positions(p).get(pos)),
                    _ => None,
                },
                fitness: percent_of_10000(ps.condition(p).fitness),
                injured: !ps.available(p, today),
            }
        })
        .collect();
    players.sort_by(|a, b| {
        a.player
            .position
            .cmp(&b.player.position)
            .then(a.player.id.cmp(&b.player.id))
    });
    dto::TrainingView {
        focus: s.focus.into(),
        intensity: s.intensity.into(),
        focuses: Focus::ALL.into_iter().map(focus_info).collect(),
        intensities: Intensity::ALL.into_iter().map(intensity_info).collect(),
        growth: pct(s.growth()),
        recovery: pct(f64::from(s.recovery_per_mille()) / 1000.0),
        injury_risk: pct(s.injury_risk()),
        drill_per_week: (u32::from(s.drill()) * 7 / 10).min(100) as u8,
        familiarity: fs_match::Formation::ALL
            .into_iter()
            .map(|f| dto::FormationFamiliarity {
                formation: f.into(),
                familiarity: (c.familiarity().get(f) / 10).min(100) as u8,
                current: Some(f) == current,
            })
            .collect(),
        players,
        positions: Position::ALL.iter().map(|p| p.code().to_owned()).collect(),
        attributes: Attr::ALL
            .iter()
            .filter(|a| a.group() != "goalkeeping")
            .chain(Attr::ALL.iter().filter(|a| a.group() == "goalkeeping"))
            .map(|a| a.label().to_owned())
            .collect(),
    }
}
