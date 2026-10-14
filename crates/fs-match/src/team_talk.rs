//! Team talks: what the manager says in the dressing room before kick-off
//! and at half-time, and how each player takes it.
//!
//! A player's reaction follows from who they are and how the match is
//! going, with no randomness: the nervous are steadied by calm and
//! overwhelmed by a rousing speech, professionals answer a demand for more,
//! hot heads sulk at criticism, and praise for a losing side breeds
//! complacency. A reaction scales the player's play until the next break,
//! by a few percent at most, like a touchline shout.

use fs_core::attributes::{Hidden, HiddenAttr};
use serde::{Deserialize, Serialize};

/// What the manager says.
///
/// The variant order is part of the save format. Only append.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TeamTalk {
    /// No pressure: go out and enjoy it.
    Calm,
    /// This is a big one: go out and win it.
    Motivate,
    /// Not good enough: I want more.
    Demand,
    /// Brilliant: keep doing exactly that.
    Praise,
    /// Stay switched on: the job is not done.
    Focus,
    /// I believe in you: turn this around.
    Faith,
}

impl TeamTalk {
    pub const ALL: [TeamTalk; 6] = [
        TeamTalk::Calm,
        TeamTalk::Motivate,
        TeamTalk::Demand,
        TeamTalk::Praise,
        TeamTalk::Focus,
        TeamTalk::Faith,
    ];
}

/// How a player takes a team talk.
///
/// The variant order is part of the save format. Only append.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum TalkReaction {
    FiredUp,
    Focused,
    Unmoved,
    Nervous,
    Complacent,
    Sulking,
}

impl TalkReaction {
    /// The multiplier on the player's play until the next break.
    pub fn boost(self) -> f64 {
        match self {
            TalkReaction::FiredUp => 1.03,
            TalkReaction::Focused => 1.015,
            TalkReaction::Unmoved => 1.0,
            TalkReaction::Nervous | TalkReaction::Complacent => 0.985,
            TalkReaction::Sulking => 0.975,
        }
    }
}

/// The match situation when the talk is given.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Situation {
    /// Our goals minus theirs.
    pub lead: i8,
    /// Half-time (false: before kick-off).
    pub half_time: bool,
}

/// How one player takes a talk, from their personality, morale (0-10000)
/// and the match situation.
pub fn reaction(hidden: &Hidden, morale: u16, talk: TeamTalk, at: Situation) -> TalkReaction {
    use TalkReaction::*;
    let t = |a: HiddenAttr| hidden.get(a);
    let pressure = t(HiddenAttr::Pressure);
    let big_games = t(HiddenAttr::ImportantMatches);
    let professional = t(HiddenAttr::Professionalism);
    let temper = t(HiddenAttr::Temperament);
    let winning = at.half_time && at.lead > 0;
    let losing = at.half_time && at.lead < 0;
    match talk {
        TeamTalk::Calm => {
            if pressure < 400 || morale < 3_500 {
                Focused
            } else if losing && professional < 400 {
                Complacent
            } else {
                Unmoved
            }
        }
        TeamTalk::Motivate => {
            if pressure < 400 {
                Nervous
            } else if big_games > 600 || professional > 700 {
                FiredUp
            } else if big_games > 400 {
                Focused
            } else {
                Unmoved
            }
        }
        TeamTalk::Demand => {
            if winning {
                if temper < 400 { Sulking } else { Unmoved }
            } else if professional > 600 {
                FiredUp
            } else if temper < 350 {
                Sulking
            } else {
                Focused
            }
        }
        TeamTalk::Praise => {
            if losing {
                if professional < 450 {
                    Complacent
                } else {
                    Unmoved
                }
            } else if (winning && professional < 300)
                || (at.half_time && at.lead == 0 && professional < 350)
            {
                Complacent
            } else if morale < 4_000 {
                FiredUp
            } else {
                Focused
            }
        }
        TeamTalk::Focus => {
            if (winning && professional > 500) || (!at.half_time && professional > 650) {
                Focused
            } else {
                Unmoved
            }
        }
        TeamTalk::Faith => {
            if losing {
                if temper > 550 && pressure > 450 {
                    FiredUp
                } else if pressure > 350 {
                    Focused
                } else {
                    Unmoved
                }
            } else if !at.half_time && pressure < 350 {
                Focused
            } else {
                Unmoved
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn person(pressure: u16, professional: u16, temper: u16) -> Hidden {
        let mut h = Hidden::default();
        h.set(HiddenAttr::Pressure, pressure);
        h.set(HiddenAttr::Professionalism, professional);
        h.set(HiddenAttr::Temperament, temper);
        h.set(HiddenAttr::ImportantMatches, 500);
        h
    }

    const PRE: Situation = Situation {
        lead: 0,
        half_time: false,
    };

    #[test]
    fn players_take_talks_as_their_character_says() {
        let nervous = person(200, 500, 500);
        let pro = person(600, 800, 600);
        let hothead = person(600, 300, 200);
        assert_eq!(
            reaction(&nervous, 5_000, TeamTalk::Calm, PRE),
            TalkReaction::Focused
        );
        assert_eq!(
            reaction(&nervous, 5_000, TeamTalk::Motivate, PRE),
            TalkReaction::Nervous
        );
        assert_eq!(
            reaction(&pro, 5_000, TeamTalk::Motivate, PRE),
            TalkReaction::FiredUp
        );
        let losing = Situation {
            lead: -1,
            half_time: true,
        };
        assert_eq!(
            reaction(&pro, 5_000, TeamTalk::Demand, losing),
            TalkReaction::FiredUp
        );
        assert_eq!(
            reaction(&hothead, 5_000, TeamTalk::Demand, losing),
            TalkReaction::Sulking
        );
        assert_eq!(
            reaction(&hothead, 5_000, TeamTalk::Praise, losing),
            TalkReaction::Complacent
        );
        let winning = Situation {
            lead: 2,
            half_time: true,
        };
        assert_eq!(
            reaction(&pro, 5_000, TeamTalk::Focus, winning),
            TalkReaction::Focused
        );
        assert_eq!(
            reaction(&hothead, 5_000, TeamTalk::Demand, winning),
            TalkReaction::Sulking
        );
    }

    #[test]
    fn no_reaction_is_worth_more_than_a_shout() {
        for r in [
            TalkReaction::FiredUp,
            TalkReaction::Focused,
            TalkReaction::Unmoved,
            TalkReaction::Nervous,
            TalkReaction::Complacent,
            TalkReaction::Sulking,
        ] {
            assert!((0.97..=1.035).contains(&r.boost()));
        }
    }
}
