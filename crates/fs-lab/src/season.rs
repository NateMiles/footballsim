//! Simulating whole league seasons with fresh generated worlds.

use fs_core::{GameDate, Seed};
use fs_gen::{GenConfig, generate};
use fs_match::{MatchInput, MatchResult, Tactic, TeamSheet, TeamStrength, Tier, tier1, tier2};
use fs_world::competition::double_round_robin;
use fs_world::{ClubId, World, matchday};

/// One simulated match plus what went into it.
#[derive(Debug, Clone)]
pub struct MatchRecord {
    pub home: usize,
    pub away: usize,
    pub strength: [TeamStrength; 2],
    pub tactics: [Tactic; 2],
    pub result: MatchResult,
}

/// One season of one league.
#[derive(Debug, Clone)]
pub struct SeasonOutcome {
    pub matches: Vec<MatchRecord>,
    /// Final points per team, in team order.
    pub points: Vec<u32>,
}

/// Clubs in the calibration league.
pub const LEAGUE_SIZE: usize = 20;

/// Which engine, with which constants.
#[derive(Debug, Clone)]
pub enum Engine {
    Event(tier1::Params),
    Statistical(tier2::Coefficients),
}

impl Engine {
    /// The calibrated engine for a tier.
    pub fn calibrated(tier: Tier) -> Self {
        match tier {
            Tier::Event => Engine::Event(tier1::Params::default()),
            Tier::Statistical => Engine::Statistical(tier2::Coefficients::default()),
        }
    }

    fn play(&self, input: &MatchInput) -> MatchResult {
        match self {
            Engine::Event(p) => tier1::play_with(input, p),
            Engine::Statistical(c) => tier2::play_with(input, c),
        }
    }
}

/// A one-nation world of 100 clubs. Its 20 strongest clubs (the first 20)
/// form a realistic top division, which is what the calibration targets
/// describe.
pub fn league_world(seed: u64) -> World {
    generate(&GenConfig {
        seed: Seed(seed),
        start: GameDate::from_ymd(2026, 7, 1).expect("valid date"),
        nations: 1,
        clubs_per_nation: 100,
        players_per_club: 25,
        free_agents: 0,
        human_clubs: Vec::new(),
    })
}

/// Simulates `seasons` double-round-robin seasons. Season `s` uses a
/// freshly generated league with seed `first_seed + s`, so each season has
/// different clubs. Matches run in parallel on native targets; results do
/// not depend on thread count.
pub fn simulate_seasons(first_seed: u64, seasons: u32, tier: Tier) -> Vec<SeasonOutcome> {
    simulate_seasons_with(first_seed, seasons, &Engine::calibrated(tier))
}

/// [`simulate_seasons`] with explicit engine constants.
pub fn simulate_seasons_with(first_seed: u64, seasons: u32, engine: &Engine) -> Vec<SeasonOutcome> {
    (0..u64::from(seasons))
        .map(|s| simulate_season(&league_world(first_seed + s), engine))
        .collect()
}

/// Simulates one season between the first [`LEAGUE_SIZE`] clubs of
/// `world` (or all of them, if fewer).
pub fn simulate_season(world: &World, engine: &Engine) -> SeasonOutcome {
    let n = world.clubs().len().min(LEAGUE_SIZE);
    let sheets: Vec<TeamSheet> = (0..n)
        .map(|c| matchday::team_sheet(world, ClubId::new(c as u32)))
        .collect();
    let strengths: Vec<TeamStrength> = sheets.iter().map(TeamStrength::of).collect();
    let fixtures: Vec<(usize, usize)> = double_round_robin(n).into_iter().flatten().collect();
    let mut results: Vec<Option<MatchResult>> = vec![None; fixtures.len()];
    fs_core::par::for_each_heavy(&mut results, |i, slot| {
        let (h, a) = fixtures[i];
        let input = MatchInput {
            home: sheets[h].clone(),
            away: sheets[a].clone(),
            neutral: false,
            seed: world.seed(),
            key: i as u64,
        };
        *slot = Some(engine.play(&input));
    });
    let mut points = vec![0u32; n];
    let matches = fixtures
        .iter()
        .zip(results)
        .map(|(&(h, a), r)| {
            let result = r.expect("every fixture played");
            match result.outcome() {
                1 => points[h] += 3,
                0 => {
                    points[h] += 1;
                    points[a] += 1;
                }
                _ => points[a] += 3,
            }
            MatchRecord {
                home: h,
                away: a,
                strength: [strengths[h], strengths[a]],
                tactics: [sheets[h].tactic, sheets[a].tactic],
                result,
            }
        })
        .collect();
    SeasonOutcome { matches, points }
}
