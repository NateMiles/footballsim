//! Personalities and influence in the dressing room.
//!
//! A personality is a label read from a player's hidden traits; it never
//! changes anything itself, it only names what the traits do elsewhere
//! (how players react to talks, how much playing time or ambition matter
//! to them, whether they hand in transfer requests). Influence decides
//! whose mood spreads through a squad.

use crate::attributes::{Attr, Attributes, Hidden, HiddenAttr};
use crate::ids::{ClubId, PlayerId};
use crate::people::age_years;
use crate::world::World;

/// The most telling trait of a player's character.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Personality {
    /// Professional and even-tempered.
    ModelProfessional,
    /// Quick to anger; reacts strongly to praise and criticism.
    Volatile,
    /// Wants to play at the highest level and be paid for it.
    Ambitious,
    /// Attached to the club; rarely asks to leave.
    Loyal,
    /// Works hard and takes criticism well.
    Professional,
    /// Content where they are.
    Unambitious,
    /// Does the minimum and resents being pushed.
    Casual,
    /// A natural leader.
    Leader,
    /// Thrives in big matches.
    BigGame,
    /// Feels the pressure.
    Nervous,
    /// Nothing stands out.
    Balanced,
}

impl Personality {
    /// Reads the label from a player's traits.
    pub fn of(hidden: &Hidden, attributes: &Attributes) -> Self {
        let h = |a| hidden.get(a);
        let professionalism = h(HiddenAttr::Professionalism);
        let temperament = h(HiddenAttr::Temperament);
        if professionalism >= 750 && temperament >= 600 {
            Personality::ModelProfessional
        } else if temperament <= 250 {
            Personality::Volatile
        } else if h(HiddenAttr::Ambition) >= 720 {
            Personality::Ambitious
        } else if h(HiddenAttr::Loyalty) >= 720 {
            Personality::Loyal
        } else if professionalism >= 700 {
            Personality::Professional
        } else if attributes.get(Attr::Leadership) >= 750 {
            Personality::Leader
        } else if h(HiddenAttr::Ambition) <= 280 {
            Personality::Unambitious
        } else if professionalism <= 280 {
            Personality::Casual
        } else if h(HiddenAttr::Pressure) >= 750 {
            Personality::BigGame
        } else if h(HiddenAttr::Pressure) <= 250 {
            Personality::Nervous
        } else {
            Personality::Balanced
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Personality::ModelProfessional => "Model professional",
            Personality::Volatile => "Volatile",
            Personality::Ambitious => "Ambitious",
            Personality::Loyal => "Loyal",
            Personality::Professional => "Professional",
            Personality::Unambitious => "Unambitious",
            Personality::Casual => "Casual",
            Personality::Leader => "Leader",
            Personality::BigGame => "Big-game player",
            Personality::Nervous => "Nervous",
            Personality::Balanced => "Balanced",
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            Personality::ModelProfessional => {
                "Works hard, keeps calm and takes criticism on board."
            }
            Personality::Volatile => "Reacts strongly to praise and criticism alike.",
            Personality::Ambitious => "Wants to play at the highest level, and to be paid like it.",
            Personality::Loyal => "Attached to the club; rarely asks to leave.",
            Personality::Professional => "Dedicated and responds well to being pushed.",
            Personality::Unambitious => "Happy where they are; rarely restless.",
            Personality::Casual => "Does the minimum and resents being criticised.",
            Personality::Leader => "A voice the rest of the squad listens to.",
            Personality::BigGame => "Rises to the big occasions.",
            Personality::Nervous => "Feels the pressure in big moments.",
            Personality::Balanced => "Nothing about their character stands out.",
        }
    }
}

/// Personality traits as fractions, 0-1.
pub(crate) fn trait_of(world: &World, player: PlayerId, attr: HiddenAttr) -> f64 {
    f64::from(world.players().hidden(player).get(attr)) / 1000.0
}

/// A player's weight in the dressing room, 0-1000: leadership, standing
/// in the team, age, years at the club, and the armband.
pub fn influence(world: &World, club: ClubId, player: PlayerId) -> u16 {
    influence_with(world, club, player, world.players().ability(player))
}

/// [`influence`], given the player's ability.
pub(crate) fn influence_with(world: &World, club: ClubId, player: PlayerId, ability: u16) -> u16 {
    let ps = world.players();
    let leadership = f64::from(ps.attributes(player).get(Attr::Leadership));
    let ability = f64::from(ability);
    let age = age_years(ps.bio(player).birth_date, world.date());
    let seniority = ((age - 18.0) / 12.0).clamp(0.0, 1.0) * 1000.0;
    let seasons = ps
        .career(player)
        .iter()
        .rev()
        .take_while(|c| c.club == Some(club))
        .count() as f64;
    let mut v = 0.4 * leadership + 0.3 * ability + 0.3 * seniority + (seasons * 30.0).min(150.0);
    if world.clubs()[club].captain == Some(player) {
        v += 150.0;
    }
    (v * 0.8).clamp(0.0, 1000.0) as u16
}

/// How many players lead a dressing room.
pub const LEADERS: usize = 3;

/// The club's most influential players, most influential first: senior
/// squad members (18 and over) only.
pub fn leaders(world: &World, club: ClubId) -> Vec<PlayerId> {
    leaders_with(world, club, |p| world.players().ability(p))
}

/// [`leaders`], given a way to look up abilities.
pub(crate) fn leaders_with(
    world: &World,
    club: ClubId,
    ability: impl Fn(PlayerId) -> u16,
) -> Vec<PlayerId> {
    let today = world.date();
    let mut v: Vec<(u16, PlayerId)> = world.clubs()[club]
        .squad()
        .iter()
        .filter(|&&p| age_years(world.players().bio(p).birth_date, today) >= 18.0)
        .map(|&p| (influence_with(world, club, p, ability(p)), p))
        .collect();
    v.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
    v.into_iter().take(LEADERS).map(|(_, p)| p).collect()
}

/// The player the club would give the armband: a leader among its best
/// and most experienced players.
pub fn natural_captain(world: &World, club: ClubId) -> Option<PlayerId> {
    let ps = world.players();
    let today = world.date();
    let mut senior: Vec<PlayerId> = world.clubs()[club]
        .squad()
        .iter()
        .copied()
        .filter(|&p| age_years(ps.bio(p).birth_date, today) >= 18.0)
        .collect();
    // Among the regular starters.
    senior.sort_by(|&a, &b| ps.ability(b).cmp(&ps.ability(a)).then(a.cmp(&b)));
    senior.truncate(14);
    senior.into_iter().max_by(|&a, &b| {
        let score = |p: PlayerId| {
            let age = age_years(ps.bio(p).birth_date, today);
            u32::from(ps.attributes(p).get(Attr::Leadership)) * 6
                + u32::from(ps.ability(p)) * 2
                + ((age - 18.0).clamp(0.0, 12.0) * 160.0) as u32
        };
        score(a).cmp(&score(b)).then(b.cmp(&a))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn labels_follow_traits() {
        let attrs = Attributes::default();
        let mut h = Hidden::default();
        assert_eq!(Personality::of(&h, &attrs), Personality::Balanced);
        h.set(HiddenAttr::Temperament, 100);
        assert_eq!(Personality::of(&h, &attrs), Personality::Volatile);
        h.set(HiddenAttr::Professionalism, 800);
        h.set(HiddenAttr::Temperament, 700);
        assert_eq!(Personality::of(&h, &attrs), Personality::ModelProfessional);
    }
}
