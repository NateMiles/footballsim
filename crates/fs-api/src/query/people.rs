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
            Talk::Ask => dto::Talk::Ask,
            Talk::Ambitions => dto::Talk::Ambitions,
            Talk::Motivate => dto::Talk::Motivate,
            Talk::Discipline => dto::Talk::Discipline,
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
            dto::Talk::Ask => Talk::Ask,
            dto::Talk::Ambitions => Talk::Ambitions,
            dto::Talk::Motivate => Talk::Motivate,
            dto::Talk::Discipline => Talk::Discipline,
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

/// What can be said, and whether it can be said to `player` today.
fn talks(world: &World, player: Option<PlayerId>) -> Vec<dto::TalkInfo> {
    let can = player.is_none_or(|p| {
        world
            .relations()
            .last_talk(p)
            .is_none_or(|d| world.date().days_since(d) >= TALK_INTERVAL)
    });
    let promised = |football: bool| {
        player.is_some_and(|p| {
            world.relations().promises().iter().any(|x| {
                x.player == p
                    && matches!(x.kind, fs_world::talk::PromiseKind::Football { .. }) == football
            })
        })
    };
    Talk::ALL
        .into_iter()
        .map(|t| {
            let (name, description, group) = match t {
                Talk::Ask => (
                    "How are you?",
                    "Ask how they are and what is on their mind.",
                    "listen",
                ),
                Talk::Ambitions => ("Ambitions", "Ask where they see their future.", "listen"),
                Talk::Praise => (
                    "Praise form",
                    "Tell them they have been playing well.",
                    "encourage",
                ),
                Talk::Reassure => (
                    "Reassure",
                    "Tell them they are an important part of your plans.",
                    "encourage",
                ),
                Talk::Motivate => (
                    "Motivate",
                    "Fire them up for the next match. Some thrive on it; some tighten up.",
                    "encourage",
                ),
                Talk::Criticise => (
                    "Criticise form",
                    "Tell them they must do better. Some respond, some sulk.",
                    "challenge",
                ),
                Talk::Discipline => (
                    "Attitude",
                    "Tell them their attitude in training must improve.",
                    "challenge",
                ),
                Talk::PromiseFootball => (
                    "Promise football",
                    "Promise more first-team football over the next two months.",
                    "promise",
                ),
                Talk::PromiseContract => (
                    "Promise a contract",
                    "Promise an improved contract within two months.",
                    "promise",
                ),
            };
            let available = match t {
                _ if t.listening() => true,
                Talk::PromiseFootball => can && !promised(true),
                Talk::PromiseContract => can && !promised(false),
                _ => can,
            };
            dto::TalkInfo {
                talk: t.into(),
                name: name.into(),
                description: description.into(),
                group: group.into(),
                line: talk_line(t).into(),
                listening: t.listening(),
                available,
            }
        })
        .collect()
}

/// What the manager says.
pub fn talk_line(t: Talk) -> &'static str {
    match t {
        Talk::Ask => "How are you getting on? Anything on your mind?",
        Talk::Ambitions => "Where do you see yourself in a few years?",
        Talk::Praise => "You've been brilliant lately. Keep it up.",
        Talk::Reassure => "You're a big part of my plans here.",
        Talk::Motivate => "Big one coming up. I need you at your very best.",
        Talk::Criticise => "Your performances haven't been good enough. I expect more.",
        Talk::Discipline => "Your attitude in training has to improve.",
        Talk::PromiseFootball => "You'll get more games. You have my word.",
        Talk::PromiseContract => "We'll sort out a new contract for you. Soon.",
    }
}

/// How a player's face looks after a talk.
pub fn reaction_expression(r: Reaction) -> dto::Expression {
    match r {
        Reaction::Delighted => dto::Expression::Delighted,
        Reaction::Pleased => dto::Expression::Happy,
        Reaction::Determined => dto::Expression::Determined,
        Reaction::Indifferent => dto::Expression::Neutral,
        Reaction::Sceptical => dto::Expression::Sceptical,
        Reaction::Annoyed => dto::Expression::Annoyed,
        Reaction::Angry => dto::Expression::Angry,
    }
}

/// What a player says about the biggest thing on their mind.
fn on_their_mind(f: Factor) -> &'static str {
    let up = f.effect > 0;
    match f.reason {
        Reason::PlayingTime if up => "I'm playing, I'm enjoying it. No complaints from me.",
        Reason::PlayingTime => "Honestly? I want to play more. I'm sitting on the bench too often.",
        Reason::Wages if up => "The club looks after me. I appreciate that.",
        Reason::Wages => "I'll be straight with you: I think I'm worth more than I'm paid.",
        Reason::Ambition => "I want to play at the very top. I'm not sure I can do that here.",
        Reason::Results if up => "We're flying. The dressing room is buzzing.",
        Reason::Results => "The results get to me. We should be doing much better.",
        Reason::Prestige => "It's an honour to wear this shirt. I don't take it for granted.",
        Reason::Future => "My contract's running down and nobody has talked to me about it.",
        Reason::Captain => "Wearing the armband means everything to me.",
        Reason::DressingRoom if up => "Good group of lads. The senior players set the tone.",
        Reason::DressingRoom => {
            "There's some unrest among the senior lads. It's not a great atmosphere."
        }
        Reason::Listed => "I didn't ask to be put up for sale. That hurt.",
        Reason::RequestPending => "I'm still waiting for an answer about my future.",
        Reason::Memory(m) => match m {
            Feeling::PromiseBroken => {
                "You promised me something and didn't deliver. I haven't forgotten."
            }
            Feeling::PromiseMade => "I'm holding on to what you promised me.",
            Feeling::PromiseKept => "You kept your word. That means a lot.",
            Feeling::UnfairCriticism | Feeling::CriticisedPublicly => {
                "What you said stung. I didn't think it was fair."
            }
            Feeling::Praised | Feeling::PraisedPublicly | Feeling::BackedPublicly => {
                "What you said about me gave me a real lift."
            }
            Feeling::Demoted | Feeling::LostCaptaincy => {
                "I'm still not happy about my role changing."
            }
            Feeling::BlockedMove | Feeling::RequestRefused => {
                "I wanted that move. I still think about it."
            }
            Feeling::Unsettled => "Hearing I might be sold has unsettled me.",
            _ => "I'm fine, boss. Just want to keep working.",
        },
    }
}

/// The player's answer, in their words.
pub fn reply_text(world: &World, player: PlayerId, talk: Talk, reaction: Reaction) -> String {
    let ps = world.players();
    let ambition = f64::from(
        ps.hidden(player)
            .get(fs_core::attributes::HiddenAttr::Ambition),
    ) / 1000.0;
    match (talk, reaction) {
        (Talk::Ask, Reaction::Sceptical) => {
            "Why ask now? You didn't keep your word last time.".to_owned()
        }
        (Talk::Ask, _) => {
            let mut lines = Vec::new();
            if let Some(a) = happiness::assess(world, player) {
                for f in a.factors.iter().take(2) {
                    lines.push(on_their_mind(*f));
                }
            }
            if lines.is_empty() {
                "All good, boss. Just want to keep working.".to_owned()
            } else {
                lines.dedup();
                lines.join(" ")
            }
        }
        (Talk::Ambitions, Reaction::Sceptical) => {
            if world.relations().request(player).is_some() {
                "You know what I want. I want to move on.".to_owned()
            } else {
                "I'll be honest: I think I need a bigger club to reach my level.".to_owned()
            }
        }
        (Talk::Ambitions, _) => {
            let mut s = if ambition > 0.65 {
                "I want to win the biggest trophies, and I want to do it here.".to_owned()
            } else if ambition > 0.35 {
                "I'm happy here. I want to keep improving and help the club climb.".to_owned()
            } else {
                "I just want to play football and enjoy it, boss.".to_owned()
            };
            if ps
                .contract(player)
                .is_some_and(|c| c.expires.days_since(world.date()) < 365)
            {
                s.push_str(" And at some point I'd like to talk about my contract.");
            }
            s
        }
        (Talk::Motivate, Reaction::Determined) => {
            "I'm ready. I'll run through walls for this team.".into()
        }
        (Talk::Motivate, Reaction::Pleased) => "Got it, boss. I'll be ready.".into(),
        (Talk::Motivate, Reaction::Annoyed) => {
            "I know it's a big game. I don't need reminding.".into()
        }
        (Talk::Motivate, _) => "You said that last week. I'm already up for it.".into(),
        (Talk::Discipline, Reaction::Angry) => {
            "My attitude? I'm first in and last out every day!".into()
        }
        (Talk::Discipline, Reaction::Determined) => "Fair enough. I'll sharpen up.".into(),
        (Talk::Discipline, _) => "Right. If you say so.".into(),
        (Talk::Praise, Reaction::Delighted) => "Thanks, boss! That means a lot.".into(),
        (Talk::Praise, Reaction::Pleased) => "Cheers. I'm really enjoying my football.".into(),
        (Talk::Praise, Reaction::Sceptical) => "I know I haven't been at my best, boss.".into(),
        (Talk::Praise, _) => "Thanks. You've said.".into(),
        (Talk::Criticise, Reaction::Determined) => "You're right. I'll show you.".into(),
        (Talk::Criticise, Reaction::Angry) => {
            "Are you serious? I've been one of our best players!".into()
        }
        (Talk::Criticise, _) => "I don't think that's fair.".into(),
        (Talk::Reassure, Reaction::Pleased) => "Good to hear. I want to be part of this.".into(),
        (Talk::Reassure, _) => "Words are nice. I'd rather be playing.".into(),
        (Talk::PromiseFootball | Talk::PromiseContract, Reaction::Pleased) => {
            "I'll hold you to that.".into()
        }
        (Talk::PromiseFootball, Reaction::Indifferent) => "I'm playing plenty, boss.".into(),
        (Talk::PromiseContract, Reaction::Indifferent) => "I'm happy with my deal for now.".into(),
        (_, Reaction::Sceptical) => "Like last time? Forgive me if I don't believe it.".into(),
        _ => reaction_text(talk, reaction).into(),
    }
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
            Feeling::PraisedPublicly => "Praised by the manager in the press",
            Feeling::CriticisedPublicly => "Criticised by the manager in public",
            Feeling::BackedPublicly => "Backed by the manager in public",
            Feeling::BlockedMove => "Told in public they are not for sale",
            Feeling::Unsettled => "Unsettled by talk of a sale",
            Feeling::KeptGrounded => "Told to keep their feet on the ground",
            Feeling::Listened => "Glad the manager asked how they are",
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
        talks: talks(world, Some(player)),
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
        talks: talks(world, None),
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
