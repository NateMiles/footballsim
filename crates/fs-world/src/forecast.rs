//! Forecasts: how the rest of a league season might go.
//!
//! Each remaining fixture's expected goals come from the tier 2 model and
//! today's team sheets; whole seasons are then sampled thousands of times.
//! Forecasts read the world and never change it, and use their own random
//! stream, so asking for one has no effect on the game.

use crate::ids::{ClubId, LeagueId};
use crate::matchday::team_sheet;
use crate::world::World;
use fs_core::Stream;
use fs_match::tier2::{Coefficients, expected_goals, sample_score};
use fs_match::{MatchInput, TeamSheet};

/// How a league season might end.
#[derive(Debug, Clone, PartialEq)]
pub struct Projection {
    pub runs: u32,
    /// One row per club, in table order today.
    pub rows: Vec<ProjectedRow>,
}

/// One club's outlook.
#[derive(Debug, Clone, PartialEq)]
pub struct ProjectedRow {
    pub club: ClubId,
    /// Points so far.
    pub points: u16,
    pub expected_points: f64,
    /// Chance of finishing in each position, first first.
    pub positions: Vec<f64>,
}

impl ProjectedRow {
    /// Chance of finishing in positions `from` to `to` (1-based, inclusive).
    pub fn chance(&self, from: usize, to: usize) -> f64 {
        self.positions
            .iter()
            .enumerate()
            .filter(|(i, _)| (from..=to).contains(&(i + 1)))
            .map(|(_, p)| p)
            .sum()
    }

    /// The position the club finishes in on average.
    pub fn expected_position(&self) -> f64 {
        self.positions
            .iter()
            .enumerate()
            .map(|(i, p)| (i + 1) as f64 * p)
            .sum()
    }
}

/// Samples the rest of a league's season `runs` times.
pub fn project_league(world: &World, league: LeagueId, runs: u32) -> Option<Projection> {
    let season = world.leagues()[league].season()?;
    let table = season.table();
    let n = table.len();
    let index = |c: ClubId| table.iter().position(|r| r.club == c);
    let sheets: Vec<TeamSheet> = table.iter().map(|r| team_sheet(world, r.club)).collect();
    let coefficients = Coefficients::default();
    // Expected goals for every remaining fixture.
    let remaining: Vec<(usize, usize, [f64; 2])> = season
        .fixtures
        .iter()
        .filter(|f| f.result.is_none())
        .filter_map(|f| {
            let (h, a) = (index(f.home)?, index(f.away)?);
            let input = MatchInput::new(sheets[h].clone(), sheets[a].clone(), world.seed(), 0);
            Some((h, a, expected_goals(&input, &coefficients)))
        })
        .collect();
    let mut rng = world.seed().derive(
        Stream::Forecast,
        u64::from(league.raw()),
        u64::from(world.date().days()),
    );
    let mut counts = vec![vec![0u32; n]; n];
    let mut total_points = vec![0u64; n];
    let mut points = vec![0u32; n];
    let mut gd = vec![0i32; n];
    let mut gf = vec![0u32; n];
    let mut order: Vec<usize> = (0..n).collect();
    let runs = runs.max(1);
    for _ in 0..runs {
        for (i, r) in table.iter().enumerate() {
            points[i] = u32::from(r.points());
            gd[i] = r.goal_difference();
            gf[i] = u32::from(r.scored);
        }
        for &(h, a, lambda) in &remaining {
            let g = sample_score(&mut rng, lambda, coefficients.rho);
            let (gh, ga) = (u32::from(g[0]), u32::from(g[1]));
            match gh.cmp(&ga) {
                core::cmp::Ordering::Greater => points[h] += 3,
                core::cmp::Ordering::Equal => {
                    points[h] += 1;
                    points[a] += 1;
                }
                core::cmp::Ordering::Less => points[a] += 3,
            }
            gd[h] += gh as i32 - ga as i32;
            gd[a] += ga as i32 - gh as i32;
            gf[h] += gh;
            gf[a] += ga;
        }
        order.sort_by(|&x, &y| {
            points[y]
                .cmp(&points[x])
                .then(gd[y].cmp(&gd[x]))
                .then(gf[y].cmp(&gf[x]))
                .then(table[x].club.cmp(&table[y].club))
        });
        for (pos, &i) in order.iter().enumerate() {
            counts[i][pos] += 1;
            total_points[i] += u64::from(points[i]);
        }
    }
    let r = f64::from(runs);
    Some(Projection {
        runs,
        rows: table
            .iter()
            .enumerate()
            .map(|(i, row)| ProjectedRow {
                club: row.club,
                points: row.points(),
                expected_points: total_points[i] as f64 / r,
                positions: counts[i].iter().map(|&c| f64::from(c) / r).collect(),
            })
            .collect(),
    })
}

/// The continental places a league's final table gives this season, by
/// cup: for each continental cup, the positions whose clubs enter it.
/// Empty for leagues below the top division or outside any continent.
pub fn continental_places(
    world: &World,
    league: LeagueId,
) -> Vec<(crate::ids::CupId, usize, usize)> {
    use crate::cup::CupScope;
    use crate::format::Source;
    let l = &world.leagues()[league];
    let top = world
        .leagues()
        .iter()
        .filter(|(_, x)| x.nation == l.nation)
        .min_by_key(|(id, x)| (x.level, *id))
        .map(|(id, _)| id);
    if top != Some(league) {
        return Vec::new();
    }
    let Some(k) = world
        .continents()
        .iter()
        .position(|c| c.nations.contains(&l.nation))
    else {
        return Vec::new();
    };
    let ranking = world.continents()[k].ranking();
    let Some(rank) = ranking
        .iter()
        .position(|&n| n == l.nation)
        .map(|r| r as u16 + 1)
    else {
        return Vec::new();
    };
    let mut out = Vec::new();
    let mut taken = 0usize;
    for (id, cup) in world.cups().iter() {
        if cup.scope != CupScope::Continent(k as u16) {
            continue;
        }
        let mut last = 0usize;
        for e in &cup.format.entries {
            if let Source::LeaguePlaces { ranks, places } = e.source
                && (ranks[0]..=ranks[1]).contains(&rank)
            {
                last = last.max(usize::from(places[1]));
            }
        }
        if last > taken {
            out.push((id, taken + 1, last));
            taken = last;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chances_sum_over_positions() {
        let row = ProjectedRow {
            club: ClubId::new(0),
            points: 0,
            expected_points: 0.0,
            positions: vec![0.5, 0.3, 0.2],
        };
        assert!((row.chance(1, 3) - 1.0).abs() < 1e-12);
        assert!((row.chance(2, 3) - 0.5).abs() < 1e-12);
        assert!((row.expected_position() - 1.7).abs() < 1e-12);
    }
}
