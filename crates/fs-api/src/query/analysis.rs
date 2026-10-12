//! Post-match analysis and season statistics by competition.

use super::refs::*;
use crate::dto;
use fs_core::GameDate;
use fs_match::analysis::MatchAnalysis;
use fs_match::{ActionKind, Outcome, PlayerMatchStats};
use fs_world::fixture::FixtureRef;
use fs_world::people::season_year;
use fs_world::records::{self, MatchRecord};
use fs_world::{ClubId, CupId, LeagueId, World};
use std::collections::BTreeMap;

/// Links shown in a pass network.
const LINKS: usize = 24;

/// The fixture a key names.
pub fn fixture_ref(key: dto::FixtureKey) -> FixtureRef {
    match key.kind {
        dto::CompetitionKind::League => FixtureRef::League {
            league: LeagueId::new(key.id),
            index: key.index,
        },
        dto::CompetitionKind::Cup => FixtureRef::Cup {
            cup: CupId::new(key.id),
            index: key.index,
        },
    }
}

fn kind_name(k: ActionKind) -> &'static str {
    match k {
        ActionKind::Header => "header",
        ActionKind::Penalty => "penalty",
        ActionKind::FreeKick => "free kick",
        _ => "shot",
    }
}

fn outcome_name(o: Outcome) -> &'static str {
    match o {
        Outcome::Goal => "goal",
        Outcome::Saved => "saved",
        Outcome::Blocked => "blocked",
        _ => "missed",
    }
}

fn line(world: &World, p: &PlayerMatchStats) -> dto::PlayerLine {
    dto::PlayerLine {
        player: player_ref(world, p.player),
        started: p.started,
        minutes: u32::from(p.minutes),
        goals: u32::from(p.goals),
        assists: u32::from(p.assists),
        shots: u32::from(p.shots),
        xg: p.xg,
        passes: u32::from(p.passes),
        passes_completed: u32::from(p.passes_completed),
        key_passes: u32::from(p.key_passes),
        dribbles_won: u32::from(p.dribbles_won),
        tackles: u32::from(p.tackles),
        interceptions: u32::from(p.interceptions),
        rating: p.rating,
    }
}

fn side_view(world: &World, club: ClubId, a: &MatchAnalysis, s: usize) -> dto::SideAnalysisView {
    let side = &a.sides[s];
    dto::SideAnalysisView {
        club: club_ref(world, club),
        touches: side.touches.iter().map(|b| b.to_vec()).collect(),
        nodes: side
            .positions
            .iter()
            .map(|p| dto::NetworkNode {
                player: player_ref(world, p.player),
                band: p.band,
                lane: p.lane,
                involvements: p.involvements,
            })
            .collect(),
        links: side
            .passes
            .iter()
            .take(LINKS)
            .map(|l| dto::NetworkLink {
                from: l.from.raw(),
                to: l.to.raw(),
                count: l.count,
            })
            .collect(),
    }
}

/// A human-managed club's match analysed, if its record is kept.
pub fn match_analysis(world: &World, r: FixtureRef) -> Option<dto::MatchAnalysisView> {
    let rec = world.record(r)?;
    let a = &rec.analysis;
    let mut totals = [0.0f32; 2];
    let mut xg = vec![dto::XgPoint {
        minute: 0,
        home: 0.0,
        away: 0.0,
    }];
    for s in &a.shots {
        totals[s.side.index()] += s.xg;
        xg.push(dto::XgPoint {
            minute: s.minute,
            home: totals[0],
            away: totals[1],
        });
    }
    Some(dto::MatchAnalysisView {
        fixture: fixture(world, r),
        goals: rec.goals,
        stats: [
            crate::live::team_stats(&rec.stats[0]),
            crate::live::team_stats(&rec.stats[1]),
        ],
        shots: a
            .shots
            .iter()
            .map(|s| dto::ShotView {
                minute: s.minute,
                side: s.side.into(),
                player: player_ref(world, s.player),
                kind: kind_name(s.kind).into(),
                band: s.zone.band,
                lane: s.zone.lane,
                xg: s.xg,
                outcome: outcome_name(s.outcome).into(),
            })
            .collect(),
        xg,
        possession: a.possession.clone(),
        sides: [
            side_view(world, rec.home, a, 0),
            side_view(world, rec.away, a, 1),
        ],
        players: [0, 1].map(|s| rec.players[s].iter().map(|p| line(world, p)).collect()),
    })
}

/// Accumulates a club's matches.
#[derive(Default)]
struct Tally {
    played: u32,
    won: u32,
    drawn: u32,
    lost: u32,
    goals: [u32; 2],
    xg: [f32; 2],
    shots: [u32; 2],
    possession: f32,
    players: BTreeMap<fs_world::PlayerId, (dto::PlayerLine, f32)>,
    matches: Vec<FixtureRef>,
}

impl Tally {
    fn add(&mut self, world: &World, rec: &MatchRecord, s: usize) {
        let o = 1 - s;
        self.played += 1;
        match rec.goals[s].cmp(&rec.goals[o]) {
            core::cmp::Ordering::Greater => self.won += 1,
            core::cmp::Ordering::Equal => self.drawn += 1,
            core::cmp::Ordering::Less => self.lost += 1,
        }
        self.goals[0] += u32::from(rec.goals[s]);
        self.goals[1] += u32::from(rec.goals[o]);
        self.xg[0] += rec.stats[s].xg;
        self.xg[1] += rec.stats[o].xg;
        self.shots[0] += u32::from(rec.stats[s].shots);
        self.shots[1] += u32::from(rec.stats[o].shots);
        self.possession += rec.stats[s].possession;
        for p in &rec.players[s] {
            let l = line(world, p);
            let e = self.players.entry(p.player).or_insert_with(|| {
                (
                    dto::PlayerLine {
                        started: false,
                        minutes: 0,
                        goals: 0,
                        assists: 0,
                        shots: 0,
                        xg: 0.0,
                        passes: 0,
                        passes_completed: 0,
                        key_passes: 0,
                        dribbles_won: 0,
                        tackles: 0,
                        interceptions: 0,
                        rating: 0.0,
                        ..l.clone()
                    },
                    0.0,
                )
            });
            let t = &mut e.0;
            t.started |= l.started;
            t.minutes += l.minutes;
            t.goals += l.goals;
            t.assists += l.assists;
            t.shots += l.shots;
            t.xg += l.xg;
            t.passes += l.passes;
            t.passes_completed += l.passes_completed;
            t.key_passes += l.key_passes;
            t.dribbles_won += l.dribbles_won;
            t.tackles += l.tackles;
            t.interceptions += l.interceptions;
            // Rating sum and appearances, averaged at the end.
            t.rating += l.rating;
            e.1 += 1.0;
        }
        self.matches.push(rec.fixture);
    }

    fn view(
        self,
        world: &World,
        competition: Option<dto::CompetitionRef>,
    ) -> dto::CompetitionSeason {
        let mut players: Vec<dto::PlayerLine> = self
            .players
            .into_values()
            .map(|(mut l, apps)| {
                l.rating /= apps.max(1.0);
                l
            })
            .collect();
        players.sort_by(|a, b| {
            b.minutes
                .cmp(&a.minutes)
                .then(a.player.id.cmp(&b.player.id))
        });
        dto::CompetitionSeason {
            competition,
            played: self.played,
            won: self.won,
            drawn: self.drawn,
            lost: self.lost,
            goals_for: self.goals[0],
            goals_against: self.goals[1],
            xg_for: self.xg[0],
            xg_against: self.xg[1],
            shots_for: self.shots[0],
            shots_against: self.shots[1],
            possession: self.possession / self.played.max(1) as f32,
            players,
            matches: self
                .matches
                .iter()
                .rev()
                .map(|&r| fixture(world, r))
                .collect(),
        }
    }
}

fn competition_key(r: FixtureRef) -> (u8, u32) {
    match r {
        FixtureRef::League { league, .. } => (0, league.raw()),
        FixtureRef::Cup { cup, .. } => (1, cup.raw()),
    }
}

/// A club's season so far, overall and by competition (matches it played
/// while human-managed).
pub fn season_stats(world: &World, club: ClubId) -> dto::SeasonStatsView {
    let year = season_year(world.date());
    let start = GameDate::from_ymd(year, 7, 1).expect("valid date");
    let mut all = Tally::default();
    let mut by: BTreeMap<(u8, u32), (FixtureRef, Tally)> = BTreeMap::new();
    for rec in records::of_club(world, club, start) {
        let s = rec.side_of(club).expect("the club played");
        all.add(world, rec, s);
        by.entry(competition_key(rec.fixture))
            .or_insert_with(|| (rec.fixture, Tally::default()))
            .1
            .add(world, rec, s);
    }
    let mut competitions = vec![all.view(world, None)];
    competitions.extend(
        by.into_values()
            .map(|(r, t)| t.view(world, Some(competition_ref(world, r)))),
    );
    dto::SeasonStatsView {
        club: club_ref(world, club),
        competitions,
    }
}
