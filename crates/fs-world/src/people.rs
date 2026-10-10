//! Creating players and changing them over time: the player factory, wage
//! demands, monthly development and ageing.

use crate::attributes::{
    ATTR_MAX, Attr, Attributes, Hidden, HiddenAttr, Position, PositionRatings,
};
use crate::ids::{ClubId, NationId};
use crate::nation::Nation;
use crate::player::{CONDITION_MAX, Condition, Contract, Foot, PlayerBio, PlayerInit};
use fs_core::{GameDate, IdVec, Rng, math};

/// How old a new player is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Age {
    /// A first-team professional, 18-35, with the age mix of a settled
    /// league.
    Senior,
    /// An academy player of the given age (typically 15-17).
    Youth(u8),
}

/// Share of first-team players by age band (18-20, 21-23, 24-27, 28-31,
/// 32-35) in a long-running simulated league. New worlds start with this
/// mix so they do not lurch toward it over their first decade.
const SENIOR_AGE_BANDS: [(i32, i32, f64); 5] = [
    (18, 20, 0.18),
    (21, 23, 0.19),
    (24, 27, 0.25),
    (28, 31, 0.24),
    (32, 35, 0.14),
];

/// What kind of player to create.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayerSpec {
    pub nation: NationId,
    pub position: Position,
    /// The level of the club (or market) they belong to, 0-1000.
    pub level: i32,
    pub age: Age,
    pub club: Option<ClubId>,
    pub squad_number: Option<u8>,
}

fn clamp_attr(x: f64) -> u16 {
    x.round().clamp(1.0, f64::from(ATTR_MAX)) as u16
}

/// Last day of the season that starts in `year`: 30 June of `year + 1`.
pub fn season_end(year: i32) -> GameDate {
    GameDate::from_ymd(year + 1, 6, 30).expect("valid date")
}

/// The year the season containing `date` started (seasons run 1 July to
/// 30 June).
pub fn season_year(date: GameDate) -> i32 {
    let (y, m, _) = date.ymd();
    if m >= 7 { y } else { y - 1 }
}

/// Creates a player. Names come from the nation's name pools.
///
/// # Panics
/// Panics if the nation has no name pools.
pub fn new_player(
    nations: &IdVec<crate::ids::NationKind, Nation>,
    rng: &mut Rng,
    today: GameDate,
    spec: PlayerSpec,
) -> PlayerInit {
    let nation = &nations[spec.nation];
    let position = spec.position;
    let age = match spec.age {
        Age::Senior => {
            let weights: Vec<f64> = SENIOR_AGE_BANDS.iter().map(|b| b.2).collect();
            let (lo, hi, _) = SENIOR_AGE_BANDS[rng.weighted_index(&weights).unwrap_or(2)];
            rng.range_i32(lo, hi)
        }
        Age::Youth(age) => i32::from(age),
    };
    let (y, m, d) = today.ymd();
    let birth = GameDate::from_ymd(y - age - 1, m, d.min(28))
        .expect("valid birth date")
        .add_days(rng.below(365));

    // Overall ability on the 0-1000 scale, and the ceiling it can grow to.
    let level = f64::from(spec.level);
    let (ability, potential) = match spec.age {
        Age::Senior => {
            // Young players have not peaked; veterans have begun to decline
            // physically.
            let mut ability = rng.normal(level * 0.85 + 60.0, 70.0);
            if age < 24 {
                ability -= f64::from(24 - age) * 22.0;
            }
            let ability = ability.clamp(60.0, 950.0);
            let room = if age < 24 {
                f64::from(24 - age) * 22.0 * rng.range_f64(0.5, 1.5)
            } else {
                rng.range_f64(0.0, 30.0)
            };
            (ability, ability + room)
        }
        Age::Youth(_) => {
            let potential = rng.normal(level * 0.85 + 60.0, 110.0);
            let ability = rng.normal(230.0 + level * 0.15, 50.0).min(potential - 60.0);
            (ability.max(60.0), potential)
        }
    };

    let key = position.key_attributes();
    let secondary = position.secondary_attributes();
    let is_gk = position == Position::Goalkeeper;
    let mut attributes = Attributes::default();
    for attr in Attr::ALL {
        let weight = if key.contains(&attr) {
            1.12
        } else if secondary.contains(&attr) {
            0.95
        } else if attr.group() == "goalkeeping" {
            if is_gk { 0.8 } else { 0.12 }
        } else if is_gk && attr.group() == "technical" {
            0.45
        } else {
            0.72
        };
        let mut value = rng.normal(ability * weight, 55.0);
        if attr.group() == "physical" && age > 30 && attr != Attr::Strength {
            value -= f64::from(age - 30) * 18.0;
        }
        attributes.set(attr, clamp_attr(value));
    }

    let mut hidden = Hidden::default();
    for h in HiddenAttr::ALL {
        hidden.set(h, clamp_attr(rng.normal(500.0, 160.0)));
    }

    let mut positions = PositionRatings::default();
    positions.set(position, 20);
    for &n in position.neighbours() {
        if rng.chance(0.6) {
            positions.set(n, 8 + rng.below(10) as u8);
        }
    }

    let foot = {
        let left_sided = matches!(
            position,
            Position::DefenderLeft
                | Position::WingBackLeft
                | Position::MidfielderLeft
                | Position::AttackingMidLeft
        );
        let roll = rng.unit_f64();
        let left_p = if left_sided { 0.65 } else { 0.18 };
        if roll < left_p {
            Foot::Left
        } else if roll < left_p + 0.07 {
            Foot::Both
        } else {
            Foot::Right
        }
    };
    let tall = matches!(position, Position::Goalkeeper | Position::DefenderCentre);
    let mut height = rng
        .normal(if tall { 188.0 } else { 179.0 }, 6.0)
        .round()
        .clamp(160.0, 205.0);
    if age < 17 {
        height -= 4.0;
    }
    let weight = (height - 105.0 + rng.normal(0.0, 4.0))
        .round()
        .clamp(50.0, 105.0);
    let first_name = nation.first_names[rng.index(nation.first_names.len())];
    let last_name = nation.last_names[rng.index(nation.last_names.len())];

    let condition = Condition {
        fitness: CONDITION_MAX - rng.below(800) as u16,
        sharpness: 4_000 + rng.below(4_000) as u16,
        fatigue: rng.below(1_000) as u16,
        morale: 4_500 + rng.below(1_000) as u16,
    };
    let rating = f64::from(position.role_rating(&attributes));
    let potential = clamp_attr(potential.max(rating)).min(980);
    let contract = spec.club.map(|_| {
        let years = match spec.age {
            Age::Senior => 1 + rng.below(5) as i32,
            Age::Youth(_) => 3,
        };
        let demand = wage_demand(rating as u16, potential, age as u32);
        Contract {
            wage: if matches!(spec.age, Age::Youth(_)) {
                (demand / 2).max(100)
            } else {
                demand
            },
            expires: season_end(season_year(today) + years - 1),
        }
    });

    PlayerInit {
        bio: PlayerBio {
            first_name,
            last_name,
            birth_date: birth,
            nation: spec.nation,
            foot,
            height_cm: height as u8,
            weight_kg: weight as u8,
        },
        club: spec.club,
        attributes,
        hidden,
        positions,
        potential,
        condition,
        contract,
        squad_number: spec.squad_number,
    }
}

/// The weekly wage a player expects, from ability, potential and age.
/// Roughly exponential in ability: an elite player earns hundreds of times
/// what a lower-league one does.
pub fn wage_demand(ability: u16, potential: u16, age: u32) -> u32 {
    let mut value = f64::from(ability);
    if age < 23 {
        // Clubs pay for promise as well as current ability.
        value += (f64::from(potential) - value).max(0.0) * 0.3;
    }
    let wage = 1000.0 * math::exp((value - 400.0) / 80.0);
    ((wage / 50.0).round() * 50.0).clamp(100.0, 1_000_000.0) as u32
}

/// The highest wage a club of this reputation (0-10000) will pay one
/// player.
pub fn wage_cap(reputation: u16) -> u32 {
    wage_demand((reputation / 10 + 60).min(1000), 0, 30)
}

/// Age in years, with a fractional part, on `today`.
pub fn age_years(birth: GameDate, today: GameDate) -> f64 {
    today.days_since(birth) as f64 / 365.25
}

/// One month of development for one player: growth toward potential while
/// young, a plateau, then decline, physical attributes first.
///
/// `professionalism` and `playing_time` are 0-1; playing time is the share
/// of available minutes the player has had this season.
pub fn develop_month(
    attrs: &mut Attributes,
    best: Position,
    potential: u16,
    age: f64,
    professionalism: f64,
    playing_time: f64,
    rng: &mut Rng,
) {
    let rating = f64::from(best.role_rating(attrs));
    // Growth is roughly linear: enough to climb from a 250 baseline to
    // potential over eight years, fastest in the late teens and tailing
    // off through the early twenties. It never overshoots potential.
    let growth_age = if age < 21.0 {
        1.0
    } else if age < 24.0 {
        0.7
    } else if age < 27.0 {
        0.3
    } else {
        0.0
    };
    let gap = (f64::from(potential) - rating).max(0.0);
    let pace = (f64::from(potential) - 250.0).max(0.0) / 96.0;
    let growth = (pace
        * growth_age
        * (0.6 + 0.8 * professionalism)
        * (0.7 + 0.6 * playing_time.clamp(0.0, 1.0)))
    .min(gap * 0.15);
    let physical_decline = if age > 29.0 { (age - 29.0) * 1.0 } else { 0.0 };
    let skill_decline = if age > 32.0 { (age - 32.0) * 0.8 } else { 0.0 };
    let experience = if (26.0..32.0).contains(&age) {
        0.3
    } else {
        0.0
    };

    let key = best.key_attributes();
    let secondary = best.secondary_attributes();
    let is_gk = best == Position::Goalkeeper;
    for attr in Attr::ALL {
        let group = attr.group();
        let weight = if key.contains(&attr) {
            1.3
        } else if secondary.contains(&attr) {
            1.0
        } else if group == "goalkeeping" {
            if is_gk { 0.8 } else { 0.0 }
        } else {
            0.6
        };
        let mut delta = growth * weight + rng.normal(0.0, 1.5);
        match group {
            "physical" if attr != Attr::Strength => {
                if age < 21.0 {
                    delta *= 1.2;
                }
                delta -= physical_decline;
            }
            "mental" => delta += experience - skill_decline * 0.5,
            _ => delta -= skill_decline,
        }
        let v = f64::from(attrs.get(attr)) + delta;
        attrs.set(attr, clamp_attr(v));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::attributes::ATTR_COUNT;

    #[test]
    fn wages_rise_steeply_with_ability() {
        let low = wage_demand(300, 300, 28);
        let mid = wage_demand(550, 550, 28);
        let high = wage_demand(850, 850, 28);
        assert!(low < mid && mid < high);
        assert!(high > 50 * low, "{low} {mid} {high}");
        assert!(wage_demand(400, 800, 18) > wage_demand(400, 400, 18));
    }

    #[test]
    fn seasons() {
        let d = GameDate::from_ymd(2027, 3, 1).unwrap();
        assert_eq!(season_year(d), 2026);
        assert_eq!(season_year(GameDate::from_ymd(2026, 7, 1).unwrap()), 2026);
        assert_eq!(season_end(2026), GameDate::from_ymd(2027, 6, 30).unwrap());
    }

    fn ability_after(start: u16, potential: u16, age: f64, years: u32) -> u16 {
        let mut a = Attributes([start; ATTR_COUNT]);
        let mut rng = Rng::from_u64(1);
        for m in 0..years * 12 {
            develop_month(
                &mut a,
                Position::MidfielderCentre,
                potential,
                age + f64::from(m) / 12.0,
                0.5,
                0.8,
                &mut rng,
            );
        }
        Position::MidfielderCentre.role_rating(&a)
    }

    #[test]
    fn young_players_grow_toward_potential() {
        let r = ability_after(350, 750, 17.0, 6);
        assert!((620..=790).contains(&r), "{r}");
    }

    #[test]
    fn peak_players_hold_steady() {
        let r = ability_after(700, 700, 26.0, 3);
        assert!((670..=730).contains(&r), "{r}");
    }

    #[test]
    fn veterans_decline() {
        let r = ability_after(750, 750, 32.0, 4);
        assert!(r < 720, "{r}");
    }
}
