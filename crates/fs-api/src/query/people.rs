//! Players as people: the dressing room, moods and talks.

use super::refs::*;
use crate::dto;
use fs_world::happiness::{self, Assessment, Factor, Feeling, Reason, SquadStatus};
use fs_world::personality::{Personality, leaders};
use fs_world::talk::{self, Promise, PromiseKind, Reaction, TALK_INTERVAL, Talk};
use fs_world::{ClubId, PlayerId, World};

/// What the viewer needs to know a player well enough to judge their
/// character.
const KNOWN: u8 = 60;

impl From<SquadStatus> for dto::SquadStatus {
    fn from(s: SquadStatus) -> Self {
        match s {
            SquadStatus::Key => dto::SquadStatus::Key,
            SquadStatus::Regular => dto::SquadStatus::Regular,
            SquadStatus::Rotation => dto::SquadStatus::Rotation,
            SquadStatus::Backup => dto::SquadStatus::Backup,
            SquadStatus::Prospect => dto::SquadStatus::Prospect,
        }
    }
}

impl From<dto::SquadStatus> for SquadStatus {
    fn from(s: dto::SquadStatus) -> Self {
        match s {
            dto::SquadStatus::Key => SquadStatus::Key,
            dto::SquadStatus::Regular => SquadStatus::Regular,
            dto::SquadStatus::Rotation => SquadStatus::Rotation,
            dto::SquadStatus::Backup => SquadStatus::Backup,
            dto::SquadStatus::Prospect => SquadStatus::Prospect,
        }
    }
}

impl From<Talk> for dto::Talk {
    fn from(t: Talk) -> Self {
        match t {
            Talk::Praise => dto::Talk::Praise,
            Talk::Criticise => dto::Talk::Criticise,
            Talk::Reassure => dto::Talk::Reassure,
            Talk::PromiseFootball => dto::Talk::PromiseFootball,
            Talk::PromiseContract => dto::Talk::PromiseContract,
        }
    }
}

impl From<dto::Talk> for Talk {
    fn from(t: dto::Talk) -> Self {
        match t {
            dto::Talk::Praise => Talk::Praise,
            dto::Talk::Criticise => Talk::Criticise,
            dto::Talk::Reassure => Talk::Reassure,
            dto::Talk::PromiseFootball => Talk::PromiseFootball,
            dto::Talk::PromiseContract => Talk::PromiseContract,
        }
    }
}

impl From<Reaction> for dto::Reaction {
    fn from(r: Reaction) -> Self {
        match r {
            Reaction::Delighted => dto::Reaction::Delighted,
            Reaction::Pleased => dto::Reaction::Pleased,
            Reaction::Determined => dto::Reaction::Determined,
            Reaction::Indifferent => dto::Reaction::Indifferent,
            Reaction::Sceptical => dto::Reaction::Sceptical,
            Reaction::Annoyed => dto::Reaction::Annoyed,
            Reaction::Angry => dto::Reaction::Angry,
        }
    }
}

pub fn status_name(s: SquadStatus) -> &'static str {
    match s {
        SquadStatus::Key => "Key player",
        SquadStatus::Regular => "Regular starter",
        SquadStatus::Rotation => "Squad rotation",
        SquadStatus::Backup => "Backup",
        SquadStatus::Prospect => "Prospect",
    }
}

pub fn statuses() -> Vec<dto::StatusInfo> {
    SquadStatus::ALL
        .into_iter()
        .map(|s| dto::StatusInfo {
            status: s.into(),
            name: status_name(s).into(),
            expected: percent_of_10000(s.expected()),
        })
        .collect()
}

fn talks() -> Vec<dto::TalkInfo> {
    Talk::ALL
        .into_iter()
        .map(|t| {
            let (name, description) = match t {
                Talk::Praise => ("Praise form", "Tell them they have been playing well."),
                Talk::Criticise => (
                    "Criticise form",
                    "Tell them they must do better. Some respond, some sulk.",
                ),
                Talk::Reassure => (
                    "Reassure",
                    "Tell them they are an important part of your plans.",
                ),
                Talk::PromiseFootball => (
                    "Promise football",
                    "Promise more first-team football over the next two months.",
                ),
                Talk::PromiseContract => (
                    "Promise a contract",
                    "Promise an improved contract within two months.",
                ),
            };
            dto::TalkInfo {
                talk: t.into(),
                name: name.into(),
                description: description.into(),
            }
        })
        .collect()
}

pub fn mood_label(h: i8) -> &'static str {
    match h {
        40.. => "Very happy",
        15..=39 => "Happy",
        -14..=14 => "Content",
        -29..=-15 => "Unhappy",
        _ => "Very unhappy",
    }
}

fn reason_text(f: Factor) -> &'static str {
    let up = f.effect > 0;
    match f.reason {
        Reason::PlayingTime if up => "Happy with their playing time",
        Reason::PlayingTime => "Wants more first-team football",
        Reason::Wages if up => "Well paid",
        Reason::Wages => "Feels underpaid",
        Reason::Ambition => "Feels too good for the club",
        Reason::Results if up => "Enjoying the club's season",
        Reason::Results => "Frustrated by the club's results",
        Reason::Prestige => "Proud to play for a club of this standing",
        Reason::Future => "Unsure of the future: the contract ends soon",
        Reason::Captain => "Proud to be captain",
        Reason::DressingRoom if up => "Good mood among the senior players",
        Reason::DressingRoom => "Unrest among the senior players",
        Reason::Listed => "Unhappy to be transfer-listed",
        Reason::RequestPending => "Waiting for an answer to a transfer request",
        Reason::Memory(m) => match m {
            Feeling::Praised => "Praised by the manager",
            Feeling::Criticised => "Stung by criticism",
            Feeling::FiredUp => "Determined to answer the manager's criticism",
            Feeling::UnfairCriticism => "Criticised unfairly",
            Feeling::Reassured => "Reassured about their place",
            Feeling::Doubted => "Doubts the manager's word",
            Feeling::PromiseMade => "Waiting for a promise to be kept",
            Feeling::PromiseKept => "A promise was kept",
            Feeling::PromiseBroken => "A promise was broken",
            Feeling::RequestRefused => "Not allowed to leave",
            Feeling::RequestAccepted => "Glad the club will let them go",
            Feeling::LostCaptaincy => "Lost the captaincy",
            Feeling::Demoted => "Unhappy about a smaller role",
            Feeling::Promoted => "Pleased with a bigger role",
            Feeling::TalksFailed => "Contract talks broke down",
        },
    }
}

fn reason(f: Factor) -> dto::MoodReason {
    dto::MoodReason {
        text: reason_text(f).into(),
        effect: f.effect,
    }
}

fn mood(a: &Assessment) -> dto::Mood {
    dto::Mood {
        happiness: a.happiness,
        label: mood_label(a.happiness).into(),
        reasons: a.factors.iter().map(|&f| reason(f)).collect(),
    }
}

/// A player's character, if the viewer knows them well enough.
pub fn personality(world: &World, player: PlayerId, knowledge: u8) -> Option<dto::Personality> {
    (knowledge >= KNOWN).then(|| {
        let ps = world.players();
        let p = Personality::of(ps.hidden(player), ps.attributes(player));
        dto::Personality {
            name: p.name().into(),
            description: p.description().into(),
        }
    })
}

fn promise_view(world: &World, p: &Promise) -> dto::PromiseView {
    let (kind, text) = match p.kind {
        PromiseKind::Football { share } => (
            "football",
            format!(
                "More first-team football: {}% of the minutes",
                percent_of_10000(share)
            ),
        ),
        PromiseKind::Contract { .. } => ("contract", "An improved contract".to_owned()),
    };
    dto::PromiseView {
        player: player_ref(world, p.player),
        kind: kind.into(),
        text,
        made: date(p.made),
        due: date(p.due),
        on_track: talk::kept(world, p),
    }
}

fn request(world: &World, player: PlayerId) -> Option<dto::RequestView> {
    world.relations().request(player).map(|r| dto::RequestView {
        date: date(r.date),
        accepted: r.accepted,
    })
}

/// A player's place in the dressing room, for their own club.
pub fn player_people(world: &World, player: PlayerId) -> Option<dto::PlayerPeople> {
    let club = world.players().club(player)?;
    let a = happiness::assess(world, player)?;
    let ps = world.players();
    let status = ps.squad_status(player);
    let last = world.relations().last_talk(player);
    Some(dto::PlayerPeople {
        status: status.into(),
        playing_time: percent_of_10000(ps.playing_time(player)),
        expected: percent_of_10000(status.expected()),
        mood: mood(&a),
        captain: world.clubs()[club].captain() == Some(player),
        influential: leaders(world, club).contains(&player),
        request: request(world, player),
        promises: world
            .relations()
            .promises()
            .iter()
            .filter(|p| p.player == player)
            .map(|p| promise_view(world, p))
            .collect(),
        last_talk: last.map(date),
        can_talk: last.is_none_or(|d| world.date().days_since(d) >= TALK_INTERVAL),
    })
}

fn atmosphere(mean: i32) -> &'static str {
    match mean {
        15.. => "Excellent",
        5..=14 => "Good",
        -5..=4 => "Calm",
        -15..=-6 => "Tense",
        _ => "Poor",
    }
}

pub fn dynamics(world: &World, club: ClubId) -> dto::DynamicsView {
    let c = &world.clubs()[club];
    let ps = world.players();
    let influential = leaders(world, club);
    let mut rows: Vec<dto::DynamicsRow> = c
        .squad()
        .iter()
        .filter_map(|&p| {
            let a = happiness::assess(world, p)?;
            let status = ps.squad_status(p);
            Some(dto::DynamicsRow {
                player: player_ref(world, p),
                age: age(world, p),
                personality: Personality::of(ps.hidden(p), ps.attributes(p))
                    .name()
                    .into(),
                status: status.into(),
                playing_time: percent_of_10000(ps.playing_time(p)),
                expected: percent_of_10000(status.expected()),
                happiness: a.happiness,
                label: mood_label(a.happiness).into(),
                reason: a.factors.first().map(|&f| reason(f)),
                captain: c.captain() == Some(p),
                influential: influential.contains(&p),
                request: request(world, p),
            })
        })
        .collect();
    rows.sort_by(|a, b| {
        a.happiness
            .cmp(&b.happiness)
            .then(a.player.id.cmp(&b.player.id))
    });
    let seniors: Vec<i32> = rows
        .iter()
        .filter(|r| r.age >= 18)
        .map(|r| i32::from(r.happiness))
        .collect();
    let mean = seniors.iter().sum::<i32>() / seniors.len().max(1) as i32;
    dto::DynamicsView {
        captain: c.captain().map(|p| player_ref(world, p)),
        leaders: influential.iter().map(|&p| player_ref(world, p)).collect(),
        happiness: mean.clamp(-100, 100) as i8,
        atmosphere: atmosphere(mean).into(),
        rows,
        promises: world
            .relations()
            .promises()
            .iter()
            .filter(|p| p.club == club)
            .map(|p| promise_view(world, p))
            .collect(),
        statuses: statuses(),
        talks: talks(),
    }
}

/// What the player said, roughly.
pub fn reaction_text(talk: Talk, reaction: Reaction) -> &'static str {
    match (talk, reaction) {
        (_, Reaction::Delighted) => "Delighted. It clearly means a lot.",
        (Talk::PromiseFootball | Talk::PromiseContract, Reaction::Pleased) => {
            "Pleased, and will hold you to it."
        }
        (_, Reaction::Pleased) => "Pleased to hear it.",
        (_, Reaction::Determined) => "Took it on board and is determined to do better.",
        (Talk::PromiseFootball, Reaction::Indifferent) => "Already plays as much as they want.",
        (Talk::PromiseContract, Reaction::Indifferent) => "Is happy with the current contract.",
        (Talk::Praise, Reaction::Indifferent) => "Has heard it before.",
        (_, Reaction::Indifferent) => "Shrugged it off.",
        (Talk::Praise, Reaction::Sceptical) => "Knows they have not been playing well.",
        (Talk::Reassure, Reaction::Sceptical) => "Wants football, not words.",
        (_, Reaction::Sceptical) => "Does not believe you after a broken promise.",
        (_, Reaction::Annoyed) => "Annoyed by the criticism.",
        (_, Reaction::Angry) => "Angry: they feel they have been playing well.",
    }
}
