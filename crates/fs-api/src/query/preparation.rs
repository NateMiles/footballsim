//! Match preparation: opposition reports, predictions and season odds.

use super::refs::*;
use crate::dto;
use fs_core::{Rng, Stream};
use fs_match::tier2::{Coefficients, expected_goals};
use fs_match::{Tactic, TeamStrength, Tier};
use fs_world::matchday::{fixture_input, team_sheet, team_sheet_with};
use fs_world::staff;
use fs_world::{ClubId, FixtureRef, LeagueId, World, scouting};

/// The viewer's next match in any competition, and the opponent.
pub fn next_match(world: &World, club: ClubId) -> Option<(FixtureRef, ClubId)> {
    let r = fs_world::fixture::next_for(world, club)?;
    let f = r.get(world)?;
    Some((r, if f.home == club { f.away } else { f.home }))
}

/// A club's results this season in every competition, oldest first.
fn outcomes(world: &World, club: ClubId) -> Vec<(u8, u8, char)> {
    fs_world::fixture::of_club(world, club)
        .into_iter()
        .filter_map(|r| {
            let f = r.get(world)?;
            let res = f.result.as_ref()?;
            let home = f.home == club;
            let (us, them) = if home {
                (res.goals[0], res.goals[1])
            } else {
                (res.goals[1], res.goals[0])
            };
            let letter = match (us.cmp(&them), res.penalties) {
                (core::cmp::Ordering::Equal, Some(p)) => {
                    if (p[0] > p[1]) == home {
                        'W'
                    } else {
                        'L'
                    }
                }
                (core::cmp::Ordering::Greater, _) => 'W',
                (core::cmp::Ordering::Less, _) => 'L',
                (core::cmp::Ordering::Equal, None) => 'D',
            };
            Some((us, them, letter))
        })
        .collect()
}

/// A tactic in words.
pub fn style(t: &Tactic) -> Vec<String> {
    let pick = |v: i8, low: [&str; 2], high: [&str; 2]| -> Option<String> {
        match v {
            -2 => Some(low[1].to_owned()),
            -1 => Some(low[0].to_owned()),
            1 => Some(high[0].to_owned()),
            2 => Some(high[1].to_owned()),
            _ => None,
        }
    };
    let t = t.clamped();
    let parts: Vec<String> = [
        pick(
            t.mentality,
            ["Cautious", "Very defensive"],
            ["Attacking", "All-out attack"],
        ),
        pick(
            t.tempo,
            ["Patient build-up", "Very slow tempo"],
            ["Quick tempo", "Frantic tempo"],
        ),
        pick(t.width, ["Narrow", "Very narrow"], ["Wide", "Very wide"]),
        pick(
            t.pressing,
            ["Drop off", "Sit in a low block"],
            ["Press", "Press very high"],
        ),
        pick(
            t.defensive_line,
            ["Deep line", "Very deep line"],
            ["High line", "Very high line"],
        ),
        pick(
            t.directness,
            ["Short passing", "Tiki-taka"],
            ["Direct", "Long balls"],
        ),
    ]
    .into_iter()
    .flatten()
    .collect();
    if parts.is_empty() {
        vec!["Balanced".to_owned()]
    } else {
        parts
    }
}

/// Compares two strengths: positive when `theirs` is better.
fn edge(theirs: f64, ours: f64, blur: f64) -> i8 {
    let d = (theirs - ours) / blur;
    if d > 0.04 {
        2
    } else if d > 0.015 {
        1
    } else if d < -0.04 {
        -2
    } else if d < -0.015 {
        -1
    } else {
        0
    }
}

fn verdict(area: &str, edge: i8) -> String {
    let lower = area.to_lowercase();
    match edge {
        2 => format!("Their {lower} is much stronger than yours"),
        1 => format!("Their {lower} is a little stronger"),
        -1 => format!("Your {lower} is a little stronger"),
        -2 => format!("Your {lower} is much stronger than theirs"),
        _ => format!("Little between you in {lower}"),
    }
}

/// What the viewer's assistant knows about `opponent`.
pub fn report(world: &World, viewer: ClubId, opponent: ClubId) -> dto::OppositionReport {
    let ps = world.players();
    let ratings = staff::ratings(world, viewer);
    let sheet = team_sheet(world, opponent);
    let ours = team_sheet(world, viewer);
    let slots = sheet.tactic.formation.slots();
    let estimate = |p| scouting::quick_estimate(world, viewer, p, &ratings);
    let likely_xi: Vec<dto::ReportPlayer> = sheet
        .starters
        .iter()
        .zip(slots.iter())
        .map(|(p, s)| dto::ReportPlayer {
            player: player_ref(world, p.id),
            position: s.position.code().to_owned(),
            ability: percent_of_1000(estimate(p.id).1.value),
            goals: ps.season_stats(p.id).goals,
        })
        .collect();
    let knowledge = sheet
        .starters
        .iter()
        .map(|p| u32::from(estimate(p.id).0))
        .sum::<u32>()
        / sheet.starters.len().max(1) as u32;

    // Players to watch: the best two by estimate, the top scorer and the
    // chief creator.
    let mut key_players: Vec<dto::KeyPlayer> = Vec::new();
    let mut by_ability: Vec<&dto::ReportPlayer> = likely_xi.iter().collect();
    by_ability.sort_by_key(|p| std::cmp::Reverse(p.ability));
    for p in by_ability.iter().take(2) {
        key_players.push(dto::KeyPlayer {
            player: p.player.clone(),
            reason: format!("Their best {} by your scouts' reckoning", p.position),
        });
    }
    let squad = world.clubs()[opponent].squad();
    let top = |f: fn(&fs_world::SeasonStats) -> u16| {
        squad
            .iter()
            .copied()
            .filter(|&p| f(ps.season_stats(p)) > 0)
            .max_by_key(|&p| (f(ps.season_stats(p)), std::cmp::Reverse(p)))
    };
    if let Some(p) = top(|s| s.goals) {
        key_players.push(dto::KeyPlayer {
            player: player_ref(world, p),
            reason: format!("Top scorer: {} goals", ps.season_stats(p).goals),
        });
    }
    if let Some(p) = top(|s| s.assists) {
        key_players.push(dto::KeyPlayer {
            player: player_ref(world, p),
            reason: format!("Chief creator: {} assists", ps.season_stats(p).assists),
        });
    }
    let mut seen = std::collections::BTreeSet::new();
    key_players.retain(|k| seen.insert(k.player.id));

    // Regulars missing: the best sixteen of the squad who cannot play.
    let today = world.date();
    let mut regulars: Vec<_> = squad.to_vec();
    regulars.sort_by_key(|&p| (std::cmp::Reverse(ps.ability(p)), p));
    regulars.truncate(16);
    let absent = regulars
        .into_iter()
        .filter(|&p| !ps.available(p, today))
        .map(|p| dto::KeyPlayer {
            player: player_ref(world, p),
            reason: match ps.injury(p) {
                Some(i) => format!("Injured until {}", date(i.until)),
                None => "Suspended".to_owned(),
            },
        })
        .collect();

    // Area by area, blurred when the scouts know little.
    let (t, o) = (TeamStrength::of(&sheet), TeamStrength::of(&ours));
    let blur = if knowledge < 35 { 2.0 } else { 1.0 };
    let comparison = [
        ("Attack", t.attack, o.attack),
        ("Defence", t.defence, o.defence),
        ("Midfield", t.control, o.control),
        ("Finishing", t.finishing, o.finishing),
        ("Goalkeeping", t.keeper, o.keeper),
    ]
    .into_iter()
    .map(|(area, a, b)| {
        let e = edge(a, b, blur);
        dto::Comparison {
            area: area.to_owned(),
            edge: e,
            verdict: verdict(area, e),
        }
    })
    .collect();

    let next = next_match(world, viewer).filter(|&(_, c)| c == opponent);
    let expected = next.map(|(r, _)| {
        let xg = expected_goals(&fixture_input(world, r), &Coefficients::default());
        let f = r.get(world).expect("fixture exists");
        if f.home == opponent {
            [xg[0] as f32, xg[1] as f32]
        } else {
            [xg[1] as f32, xg[0] as f32]
        }
    });
    let results = outcomes(world, opponent);
    let mut record = dto::Record::default();
    for &(us, them, letter) in &results {
        record.played += 1;
        record.scored += u16::from(us);
        record.conceded += u16::from(them);
        match letter {
            'W' => record.won += 1,
            'L' => record.lost += 1,
            _ => record.drawn += 1,
        }
    }
    let league = world.clubs()[opponent].league();
    let position = league.and_then(|l| {
        let s = world.leagues()[l].season()?;
        s.table()
            .iter()
            .position(|r| r.club == opponent)
            .map(|p| (p + 1) as u8)
    });
    dto::OppositionReport {
        club: club_ref(world, opponent),
        fixture: next.map(|(r, _)| fixture(world, r)),
        league: league.map(|l| league_ref(world, l)),
        position,
        tactic: sheet.tactic.into(),
        style: style(&sheet.tactic),
        likely_xi,
        form: results
            .iter()
            .rev()
            .take(5)
            .rev()
            .map(|r| r.2.to_string())
            .collect(),
        record,
        key_players,
        absent,
        comparison,
        expected_goals: expected,
        knowledge: knowledge as u8,
    }
}

/// Fewest and most runs a prediction may ask for.
pub const PREDICTION_RUNS: (u32, u32) = (50, 2_000);

/// The viewer's next match played `runs` times with `tactic` (or their
/// current one), on the detailed engine. The runs use their own random
/// streams, never the one the real match will use.
pub fn predict(
    world: &World,
    club: ClubId,
    tactic: Option<Tactic>,
    runs: u32,
) -> Option<dto::Prediction> {
    let (r, _) = next_match(world, club)?;
    let mut input = fixture_input(world, r);
    let home = r.get(world)?.home == club;
    let tactic = tactic.map(Tactic::clamped).unwrap_or(if home {
        input.home.tactic
    } else {
        input.away.tactic
    });
    let sheet = team_sheet_with(world, club, tactic);
    if home {
        input.home = sheet;
    } else {
        input.away = sheet;
    }
    input.home.auto_subs = true;
    input.away.auto_subs = true;
    let mut rng: Rng = world.seed().derive(Stream::Forecast, input.key, 1);
    input.key = rng.next_u64();
    let runs = runs.clamp(PREDICTION_RUNS.0, PREDICTION_RUNS.1);
    let p = fs_match::predict::predict_input(&input, runs, Tier::Event);
    Some(dto::Prediction {
        fixture: fixture(world, r),
        tactic: tactic.into(),
        runs: p.runs,
        home_win: p.home_win as f32,
        draw: p.draw as f32,
        away_win: p.away_win as f32,
        goals: [p.home_goals as f32, p.away_goals as f32],
        xg: [p.home_xg as f32, p.away_xg as f32],
        scores: p
            .scores
            .iter()
            .take(6)
            .map(|&(goals, chance)| dto::ScoreChance {
                goals,
                chance: chance as f32,
            })
            .collect(),
    })
}

/// Seasons simulated for odds.
pub const PROJECTION_RUNS: u32 = 2_000;

/// How a league season might end.
pub fn projections(world: &World, league: LeagueId) -> dto::ProjectionsView {
    let l = &world.leagues()[league];
    let n = l.clubs().len();
    // Zones: champions, continental places, promotion and relegation.
    let mut zones: Vec<dto::ProjectionZone> = Vec::new();
    if l.level == 1 {
        zones.push(dto::ProjectionZone {
            name: "Champions".into(),
            from: 1,
            to: 1,
            cup: None,
        });
    }
    for (cup, from, to) in fs_world::forecast::continental_places(world, league) {
        zones.push(dto::ProjectionZone {
            name: world.cups()[cup].format.name.clone(),
            from: from as u8,
            to: to.min(n) as u8,
            cup: Some(cup_ref(world, cup)),
        });
    }
    let swaps = usize::from(l.swaps).min(n / 2);
    let above = world
        .leagues()
        .iter()
        .any(|(_, o)| o.nation == l.nation && o.level + 1 == l.level);
    let below = world
        .leagues()
        .iter()
        .any(|(_, o)| o.nation == l.nation && o.level == l.level + 1);
    if above && swaps > 0 {
        zones.push(dto::ProjectionZone {
            name: "Promotion".into(),
            from: 1,
            to: swaps as u8,
            cup: None,
        });
    }
    if below && swaps > 0 {
        zones.push(dto::ProjectionZone {
            name: "Relegation".into(),
            from: (n - swaps + 1) as u8,
            to: n as u8,
            cup: None,
        });
    }
    let projection = fs_world::forecast::project_league(world, league, PROJECTION_RUNS);
    let mut rows: Vec<dto::ProjectionRow> = projection
        .as_ref()
        .map(|p| {
            p.rows
                .iter()
                .enumerate()
                .map(|(i, row)| dto::ProjectionRow {
                    club: club_ref(world, row.club),
                    position: (i + 1) as u8,
                    points: row.points,
                    expected_points: row.expected_points as f32,
                    expected_position: row.expected_position() as f32,
                    zones: zones
                        .iter()
                        .map(|z| row.chance(usize::from(z.from), usize::from(z.to)) as f32)
                        .collect(),
                    positions: row.positions.iter().map(|&x| x as f32).collect(),
                })
                .collect()
        })
        .unwrap_or_default();
    // The projected table: likeliest finish first.
    rows.sort_by(|a, b| {
        a.expected_position
            .total_cmp(&b.expected_position)
            .then(a.position.cmp(&b.position))
    });
    dto::ProjectionsView {
        league: league_ref(world, league),
        season: l.season().map(|s| season_label(s.year)).unwrap_or_default(),
        runs: projection.map_or(0, |p| p.runs),
        rows,
        zones,
    }
}
