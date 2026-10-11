//! `fs-cli`: headless runner for footballsim.
//!
//! ```text
//! fs-cli gen         [--size tiny|small|medium|large] [--seed N]
//! fs-cli run         [--size ...] [--seed N] [--days N]
//! fs-cli squad       [--size ...] [--seed N] [--club N]
//! fs-cli determinism [--size ...] [--seed N] [--days N]
//! fs-cli match       [--seed N] [--home N] [--away N] [--tier 1|2]
//! fs-cli predict     [--seed N] [--home N] [--away N] [--tier 1|2] [--runs N]
//! fs-cli calibrate   [--seed N] [--seasons N] [--tier 1|2] [--out FILE]
//! fs-cli career      [--size ...] [--seed N] [--seasons N]
//! fs-cli format      [--size ...] [--seed N] [--out FILE]
//! fs-cli economy     [--size ...] [--seed N]
//! fs-cli compare-tiers [--runs N]
//! fs-cli fit-tier2   [--seed N] [--seasons N] [--out crates/fs-match/src/fitted.rs]
//! ```
//!
//! Match commands use the calibration league world; `--home` and
//! `--away` are club numbers (0 is the strongest).
//!
//! `format` writes a world's competition structure as JSON; `gen`, `run`,
//! `squad`, `career` and `economy` take `--format FILE` to play an edited
//! one instead of the default for `--size`.
//!
//! Builds for native and `wasm32-wasip1`; `scripts/determinism-check.sh`
//! runs `determinism` on both and compares the output.

use fs_core::state_hash;
use fs_gen::{GenConfig, Size, generate, scenario};
use fs_match::{MatchEvent, Side, TeamSheet, Tier};
use fs_world::attributes::{Attr, Position};
use fs_world::{Actor, ClubId, Command, Game, World, matchday};
use std::process::ExitCode;
use std::time::Instant;

struct Args {
    size: Size,
    seed: u64,
    days: u16,
    club: u32,
    home: u32,
    away: u32,
    tier: Tier,
    runs: u32,
    seasons: u32,
    out: Option<String>,
    format: Option<String>,
}

const USAGE: &str = "usage: fs-cli <gen|run|squad|determinism|match|predict|calibrate|compare-tiers|fit-tier2|career|economy|format> \
[--size tiny|small|medium|large] [--seed N] [--days N] [--club N] [--home N] [--away N] \
[--tier 1|2] [--runs N] [--seasons N] [--out FILE] [--format FILE]";

fn parse(mut args: impl Iterator<Item = String>) -> Result<(String, Args), String> {
    let cmd = args.next().ok_or_else(|| USAGE.to_string())?;
    let mut out = Args {
        size: Size::Tiny,
        seed: 1,
        days: 365,
        club: 0,
        home: 0,
        away: 1,
        tier: Tier::Event,
        runs: 10_000,
        seasons: 10,
        out: None,
        format: None,
    };
    while let Some(flag) = args.next() {
        let value = args.next().ok_or_else(|| format!("{flag} needs a value"))?;
        let bad = |_| format!("bad value for {flag}: {value}");
        match flag.as_str() {
            "--size" => {
                out.size = match value.as_str() {
                    "tiny" => Size::Tiny,
                    "small" => Size::Small,
                    "medium" => Size::Medium,
                    "large" => Size::Large,
                    _ => return Err(format!("unknown size {value}")),
                }
            }
            "--seed" => out.seed = value.parse().map_err(bad)?,
            "--days" => out.days = value.parse().map_err(bad)?,
            "--club" => out.club = value.parse().map_err(bad)?,
            "--home" => out.home = value.parse().map_err(bad)?,
            "--away" => out.away = value.parse().map_err(bad)?,
            "--runs" => out.runs = value.parse().map_err(bad)?,
            "--seasons" => out.seasons = value.parse().map_err(bad)?,
            "--out" => out.out = Some(value.clone()),
            "--format" => out.format = Some(value.clone()),
            "--tier" => {
                out.tier = match value.as_str() {
                    "1" => Tier::Event,
                    "2" => Tier::Statistical,
                    _ => return Err(format!("unknown tier {value} (use 1 or 2)")),
                }
            }
            _ => return Err(format!("unknown flag {flag}\n{USAGE}")),
        }
    }
    Ok((cmd, out))
}

fn main() -> ExitCode {
    let (cmd, args) = match parse(std::env::args().skip(1)) {
        Ok(x) => x,
        Err(e) => {
            eprintln!("{e}");
            return ExitCode::from(2);
        }
    };
    match cmd.as_str() {
        "gen" => gen_cmd(&args),
        "run" => run_cmd(&args),
        "squad" => squad_cmd(&args),
        "determinism" => determinism_cmd(&args),
        "match" => return match_cmd(&args),
        "predict" => return predict_cmd(&args),
        "calibrate" => return calibrate_cmd(&args),
        "fit-tier2" => return fit_cmd(&args),
        "compare-tiers" => compare_cmd(&args),
        "career" => career_cmd(&args),
        "economy" => economy_cmd(&args),
        "format" => return format_cmd(&args),
        _ => {
            eprintln!("{USAGE}");
            return ExitCode::from(2);
        }
    }
    ExitCode::SUCCESS
}

/// The world config: the preset for `--size`, with the competition
/// structure from `--format` if given.
///
/// # Panics
/// Panics if the format file cannot be read or parsed.
fn config(a: &Args) -> GenConfig {
    let mut cfg = GenConfig::preset(a.size, a.seed);
    if let Some(path) = &a.format {
        let text = std::fs::read_to_string(path).unwrap_or_else(|e| panic!("{path}: {e}"));
        let format: fs_world::format::WorldFormat =
            serde_json::from_str(&text).unwrap_or_else(|e| panic!("{path}: {e}"));
        cfg.format = Some(format);
    }
    cfg
}

/// Writes the competition structure as JSON.
fn format_cmd(a: &Args) -> ExitCode {
    let format = fs_gen::format_for(&config(a));
    if let Err(e) = format.validate() {
        eprintln!("invalid format: {e}");
        return ExitCode::FAILURE;
    }
    let json = serde_json::to_string_pretty(&format).expect("formats serialise");
    match &a.out {
        Some(path) => {
            if let Err(e) = std::fs::write(path, json + "\n") {
                eprintln!("{path}: {e}");
                return ExitCode::FAILURE;
            }
            println!("wrote {path}");
        }
        None => println!("{json}"),
    }
    ExitCode::SUCCESS
}

fn ms(t: Instant) -> f64 {
    t.elapsed().as_secs_f64() * 1000.0
}

fn gen_cmd(a: &Args) {
    let t = Instant::now();
    let world = generate(&config(a));
    let gen_ms = ms(t);
    let t = Instant::now();
    let snapshot = world.to_snapshot();
    let save_ms = ms(t);
    println!("nations      {}", world.nations().len());
    println!("clubs        {}", world.clubs().len());
    println!("players      {}", world.players().len());
    println!("names        {}", world.names().len());
    println!("generate     {gen_ms:.1} ms");
    println!(
        "snapshot     {:.1} MB in {save_ms:.1} ms",
        snapshot.len() as f64 / 1e6
    );
    println!("hash         {}", world.state_hash());
}

fn run_cmd(a: &Args) {
    let world = generate(&config(a));
    let players = world.players().len();
    let mut game = Game::new(world);
    let t = Instant::now();
    let mut left = a.days;
    while left > 0 {
        let step = left.min(fs_world::command::MAX_ADVANCE_DAYS);
        game.submit(Actor::System, Command::Advance { days: step })
            .expect("advance");
        left -= step;
    }
    let total = ms(t);
    println!("players      {players}");
    println!("days         {}", a.days);
    println!("total        {total:.1} ms");
    println!("per day      {:.3} ms", total / f64::from(a.days.max(1)));
    println!("date         {}", game.world().date());
    println!("hash         {}", game.world().state_hash());
}

fn squad_cmd(a: &Args) {
    let world = generate(&config(a));
    let Some(club) = world.clubs().get(ClubId::new(a.club)) else {
        eprintln!("no club {}", a.club);
        return;
    };
    let nation = &world.nations()[club.nation];
    println!(
        "{} ({}), reputation {}",
        club.name, nation.name, club.reputation
    );
    println!(
        "{:>3} {:<28} {:>3} {:<4} {:>3} {:>3} {:>3} {:>3} {:>3}",
        "No", "Name", "Age", "Pos", "Pac", "Pas", "Tck", "Fin", "Dec"
    );
    let today = world.date();
    for &p in club.squad() {
        let pl = world.players();
        let attrs = pl.attributes(p);
        let pos: Position = pl.positions(p).best();
        println!(
            "{:>3} {:<28} {:>3} {:<4} {:>3} {:>3} {:>3} {:>3} {:>3}",
            pl.squad_number(p)
                .map_or(String::from("-"), |n| n.to_string()),
            world.player_name(p),
            today.age_of(pl.bio(p).birth_date),
            pos.code(),
            attrs.display(Attr::Pace),
            attrs.display(Attr::Passing),
            attrs.display(Attr::Tackling),
            attrs.display(Attr::Finishing),
            attrs.display(Attr::Decisions),
        );
    }
}

/// Prints hashes that must be identical on every platform.
fn determinism_cmd(a: &Args) {
    let game = scenario::determinism(a.seed, a.size, a.days);
    println!("world {}", game.world().state_hash());
    println!("log   {}", state_hash(game.log()));
    println!("date  {}", game.world().date());
    // A season of matches from the evolved world, in both tiers: the match
    // engines use floating point heavily, so they must agree bit for bit
    // too.
    let world = game.world();
    let clubs = world.clubs().len().min(fs_lab::season::LEAGUE_SIZE);
    if clubs >= 2 {
        let fixtures: Vec<(usize, usize)> = fs_world::competition::double_round_robin(clubs)
            .into_iter()
            .flatten()
            .collect();
        let sheets: Vec<TeamSheet> = (0..clubs)
            .map(|c| matchday::team_sheet(world, ClubId::new(c as u32)))
            .collect();
        for tier in [Tier::Event, Tier::Statistical] {
            let results: Vec<fs_match::MatchResult> = fixtures
                .iter()
                .enumerate()
                .map(|(k, &(h, w))| {
                    let input = fs_match::MatchInput::new(
                        sheets[h].clone(),
                        sheets[w].clone(),
                        world.seed(),
                        k as u64,
                    );
                    fs_match::play(&input, tier)
                })
                .collect();
            let label = if tier == Tier::Event {
                "tier1"
            } else {
                "tier2"
            };
            println!("{label} {}", state_hash(&results));
        }
    }
}

fn tiny_world(seed: u64) -> World {
    fs_lab::league_world(seed)
}

fn sheets(world: &World, a: &Args) -> Result<(TeamSheet, TeamSheet), String> {
    let n = world.clubs().len() as u32;
    if a.home >= n || a.away >= n || a.home == a.away {
        return Err(format!(
            "--home and --away must be different clubs below {n}"
        ));
    }
    Ok((
        matchday::team_sheet(world, ClubId::new(a.home)),
        matchday::team_sheet(world, ClubId::new(a.away)),
    ))
}

fn tier_name(t: Tier) -> &'static str {
    match t {
        Tier::Event => "tier 1 (event)",
        Tier::Statistical => "tier 2 (statistical)",
    }
}

fn match_cmd(a: &Args) -> ExitCode {
    let world = tiny_world(a.seed);
    let (home, away) = match sheets(&world, a) {
        Ok(x) => x,
        Err(e) => {
            eprintln!("{e}");
            return ExitCode::from(2);
        }
    };
    let names = [
        world.clubs()[ClubId::new(a.home)].name.clone(),
        world.clubs()[ClubId::new(a.away)].name.clone(),
    ];
    let formations = [home.tactic.formation.name(), away.tactic.formation.name()];
    let input = fs_match::MatchInput::new(home, away, world.seed(), u64::from(a.runs));
    let t = Instant::now();
    let r = fs_match::play(&input, a.tier);
    let elapsed = t.elapsed().as_secs_f64() * 1e6;
    let name = |id| world.player_name(id);
    println!(
        "{} {} - {} {}   ({})",
        names[0],
        r.goals[0],
        r.goals[1],
        names[1],
        tier_name(a.tier)
    );
    println!("{:>24}   {:<8}   {}", formations[0], "", formations[1]);
    println!();
    for e in &r.events {
        let side = |s: Side| &names[s.index()];
        match e {
            MatchEvent::Goal {
                minute,
                side: s,
                scorer,
                assist,
                kind,
                xg,
            } => {
                let assist = assist
                    .map(|p| format!(", assist {}", name(p)))
                    .unwrap_or_default();
                println!(
                    "{minute:>3}'  GOAL  {}: {}{assist} ({kind:?}, xG {xg:.2})",
                    side(*s),
                    name(*scorer)
                );
            }
            MatchEvent::PenaltyMissed {
                minute,
                side: s,
                taker,
            } => println!("{minute:>3}'  pen missed  {}: {}", side(*s), name(*taker)),
            MatchEvent::Card {
                minute,
                side: s,
                player,
                red,
            } => {
                println!(
                    "{minute:>3}'  {}  {}: {}",
                    if *red { "RED " } else { "yellow" },
                    side(*s),
                    name(*player)
                );
            }
            MatchEvent::Substitution {
                minute,
                side: s,
                off,
                on,
            } => println!(
                "{minute:>3}'  sub   {}: {} for {}",
                side(*s),
                name(*on),
                name(*off)
            ),
            MatchEvent::Injury {
                minute,
                side: s,
                player,
            } => println!("{minute:>3}'  injury  {}: {}", side(*s), name(*player)),
        }
    }
    println!();
    let [h, w] = r.stats;
    let row = |label: &str, x: String, y: String| println!("{x:>10}  {label:^18}  {y:<10}");
    row(
        "Possession",
        format!("{:.0}%", h.possession * 100.0),
        format!("{:.0}%", w.possession * 100.0),
    );
    row("Shots", h.shots.to_string(), w.shots.to_string());
    row(
        "On target",
        h.shots_on_target.to_string(),
        w.shots_on_target.to_string(),
    );
    row("xG", format!("{:.2}", h.xg), format!("{:.2}", w.xg));
    row(
        "Passes",
        format!("{} ({:.0}%)", h.passes, h.pass_completion() * 100.0),
        format!("{} ({:.0}%)", w.passes, w.pass_completion() * 100.0),
    );
    row("Corners", h.corners.to_string(), w.corners.to_string());
    row("Fouls", h.fouls.to_string(), w.fouls.to_string());
    row(
        "Cards",
        format!("{}/{}", h.yellows, h.reds),
        format!("{}/{}", w.yellows, w.reds),
    );
    println!();
    for (team, players) in names.iter().zip(&r.players) {
        let best = players
            .iter()
            .filter(|p| p.minutes > 0)
            .max_by(|x, y| x.rating.total_cmp(&y.rating));
        if let Some(b) = best {
            println!("Best for {}: {} ({:.1})", team, name(b.player), b.rating);
        }
    }
    println!("\nsimulated in {elapsed:.0} µs");
    ExitCode::SUCCESS
}

fn predict_cmd(a: &Args) -> ExitCode {
    let world = tiny_world(a.seed);
    let (home, away) = match sheets(&world, a) {
        Ok(x) => x,
        Err(e) => {
            eprintln!("{e}");
            return ExitCode::from(2);
        }
    };
    let t = Instant::now();
    let p = fs_lab::predict(&home, &away, world.seed(), a.runs, a.tier);
    let secs = t.elapsed().as_secs_f64();
    println!(
        "{} v {}: {} runs, {}, {:.2} s",
        world.clubs()[ClubId::new(a.home)].name,
        world.clubs()[ClubId::new(a.away)].name,
        p.runs,
        tier_name(a.tier),
        secs
    );
    println!(
        "home win {:5.1}%   draw {:5.1}%   away win {:5.1}%",
        100.0 * p.home_win,
        100.0 * p.draw,
        100.0 * p.away_win
    );
    println!(
        "goals {:.2} - {:.2}   xG {:.2} - {:.2}",
        p.home_goals, p.away_goals, p.home_xg, p.away_xg
    );
    println!("likeliest scores:");
    for (s, prob) in p.scores.iter().take(6) {
        println!("  {}-{}  {:4.1}%", s[0], s[1], 100.0 * prob);
    }
    ExitCode::SUCCESS
}

fn calibrate_cmd(a: &Args) -> ExitCode {
    let t = Instant::now();
    let seasons = fs_lab::simulate_seasons(a.seed, a.seasons, a.tier);
    let secs = t.elapsed().as_secs_f64();
    let report = fs_lab::report(&seasons);
    let mut md = format!("# Calibration: {}\n\n", tier_name(a.tier));
    md.push_str(&report.to_markdown());
    md.push_str(&format!(
        "\nGenerated by `fs-cli calibrate --tier {} --seasons {} --seed {}`.\n",
        if a.tier == Tier::Event { 1 } else { 2 },
        a.seasons,
        a.seed
    ));
    print!("{md}");
    eprintln!("simulated in {secs:.2} s");
    if let Some(path) = &a.out
        && let Err(e) = std::fs::write(path, &md)
    {
        eprintln!("could not write {path}: {e}");
        return ExitCode::from(1);
    }
    if report.all_ok() {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    }
}

fn fit_cmd(a: &Args) -> ExitCode {
    let seasons = fs_lab::simulate_seasons(a.seed, a.seasons, Tier::Event);
    let c = fs_lab::fit::fit_tier2(&seasons);
    let literal = c.to_rust();
    match &a.out {
        Some(path) => {
            let src = format!(
                "//! Generated by `fs-cli fit-tier2 --seasons {} --seed {} --out\n//! crates/fs-match/src/fitted.rs`. Do not edit by hand: re-run it after\n//! changing tier 1.\n\nuse crate::tier2::Coefficients;\n\n/// Tier 2 coefficients fitted to tier 1.\npub const TIER2: Coefficients = {};\n",
                a.seasons, a.seed, literal
            );
            if let Err(e) = std::fs::write(path, src) {
                eprintln!("could not write {path}: {e}");
                return ExitCode::from(1);
            }
            eprintln!("wrote {path}");
        }
        None => println!("{literal}"),
    }
    ExitCode::SUCCESS
}

fn compare_cmd(a: &Args) {
    let rows = fs_lab::compare::compare_tiers(a.runs);
    println!("## Tier 1 against tier 2, fixture by fixture\n");
    println!(
        "{} runs per fixture per tier, seed-42 league (club 0 strongest).\n",
        a.runs
    );
    print!("{}", fs_lab::compare::to_markdown(&rows));
}

/// Simulates whole seasons and reports on the human manager's league and
/// on the health of the world.
fn career_cmd(a: &Args) {
    use fs_world::Status;
    use fs_world::people::{age_years, season_year};
    let mut game = Game::new(generate(&config(a)));
    // Holiday mode: the assistant handles the human club's contracts.
    let human_manager = game.world().human_managers().next();
    if let Some(m) = human_manager {
        game.submit(Actor::Manager(m), Command::SetDelegation { squad: true })
            .expect("delegate");
    }
    let w = game.world();
    let human = w.human_managers().next().and_then(|m| w.managers()[m].club);
    let league = human
        .and_then(|c| w.clubs()[c].league())
        .unwrap_or(fs_world::LeagueId::new(0));
    // Track the five most promising teenagers.
    let today = w.date();
    let mut prospects: Vec<fs_world::PlayerId> = w
        .players()
        .ids()
        .filter(|&p| {
            w.players().club(p).is_some() && age_years(w.players().bio(p).birth_date, today) < 20.0
        })
        .collect();
    prospects.sort_by_key(|&p| std::cmp::Reverse(w.players().potential(p)));
    prospects.truncate(5);
    let mut prospect_rows: Vec<Vec<u16>> = prospects
        .iter()
        .map(|&p| vec![w.players().ability(p)])
        .collect();
    let players_at_start = w.players().len();
    let mut requests_before = 0;
    println!(
        "{} clubs, {} players, {} leagues. Following {}.\n",
        w.clubs().len(),
        w.players().len(),
        w.leagues().len(),
        w.leagues()[league].name
    );

    for _ in 0..a.seasons {
        let t = Instant::now();
        let year = season_year(game.world().date());
        let target = fs_core::GameDate::from_ymd(year + 1, 7, 1).expect("valid date");
        let mut sackings = 0;
        while game.world().date() < target {
            let days = target.days_since(game.world().date()).min(366) as u16;
            let events = game
                .submit(Actor::System, Command::Advance { days })
                .expect("advance");
            sackings += events
                .iter()
                .filter(|e| matches!(e, fs_world::Event::ManagerSacked { .. }))
                .count();
        }
        let secs = t.elapsed().as_secs_f64();
        let w = game.world();
        let l = &w.leagues()[league];
        let Some(summary) = l.history().last() else {
            continue;
        };
        let club_name = |c: ClubId| &w.clubs()[c].name;
        println!(
            "== {}/{:02} {} ({secs:.2} s)",
            summary.year,
            (summary.year + 1) % 100,
            l.name
        );
        let n = summary.table.len();
        for (pos, row) in summary.table.iter().enumerate() {
            if pos < 4 || pos + 3 >= n || Some(row.club) == human {
                let marker = if Some(row.club) == human { "*" } else { " " };
                println!(
                    "  {:>2}.{marker}{:<28} {:>2}W {:>2}D {:>2}L {:>3}-{:<3} {:>3} pts",
                    pos + 1,
                    club_name(row.club),
                    row.won,
                    row.drawn,
                    row.lost,
                    row.scored,
                    row.conceded,
                    row.points()
                );
            } else if pos == 4 {
                println!("      ...");
            }
        }
        if let Some((p, goals)) = summary.top_scorer {
            println!("  Top scorer: {} ({goals})", w.player_name(p));
        }
        // World health.
        let today = w.date();
        let ps = w.players();
        let active: Vec<_> = ps
            .ids()
            .filter(|&p| ps.status(p) == Status::Active)
            .collect();
        let at_clubs: Vec<_> = active
            .iter()
            .copied()
            .filter(|&p| ps.club(p).is_some())
            .collect();
        let ages = at_clubs
            .iter()
            .map(|&p| age_years(ps.bio(p).birth_date, today))
            .sum::<f64>()
            / at_clubs.len().max(1) as f64;
        let sizes: Vec<usize> = w.clubs().iter().map(|(_, c)| c.squad().len()).collect();
        let top: Vec<u16> = w.leagues()[fs_world::LeagueId::new(0)]
            .clubs()
            .iter()
            .flat_map(|&c| {
                let mut abil: Vec<u16> = w.clubs()[c]
                    .squad()
                    .iter()
                    .map(|&p| ps.ability(p))
                    .collect();
                abil.sort_unstable_by(|a, b| b.cmp(a));
                abil.truncate(11);
                abil
            })
            .collect();
        let top_avg = top.iter().map(|&x| f64::from(x)).sum::<f64>() / top.len().max(1) as f64;
        println!(
            "  World: {} active ({} at clubs, {} free), {} retired, {} created since start; mean age {:.1}; \
             squads {}-{} (mean {:.1}); top-division best-XI ability {:.0}",
            active.len(),
            at_clubs.len(),
            active.len() - at_clubs.len(),
            ps.len() - active.len(),
            ps.len() - players_at_start,
            ages,
            sizes.iter().min().unwrap_or(&0),
            sizes.iter().max().unwrap_or(&0),
            sizes.iter().sum::<usize>() as f64 / sizes.len().max(1) as f64,
            top_avg
        );
        let mut bands = [0usize; 5];
        let mut academy = 0;
        let mut senior_age = 0.0;
        for &p in &at_clubs {
            let age = age_years(ps.bio(p).birth_date, today);
            if age < 18.0 {
                academy += 1;
                continue;
            }
            senior_age += age;
            bands[match age {
                x if x < 21.0 => 0,
                x if x < 24.0 => 1,
                x if x < 28.0 => 2,
                x if x < 32.0 => 3,
                _ => 4,
            }] += 1;
        }
        let seniors: usize = bands.iter().sum();
        let pct = |n: usize| 100.0 * n as f64 / seniors.max(1) as f64;
        println!(
            "  First team (18+): mean age {:.1}; 18-20 {:.0}%, 21-23 {:.0}%, 24-27 {:.0}%, 28-31 {:.0}%, 32+ {:.0}%. Academy: {} players",
            senior_age / seniors.max(1) as f64,
            pct(bands[0]),
            pct(bands[1]),
            pct(bands[2]),
            pct(bands[3]),
            pct(bands[4]),
            academy
        );
        println!("  Economy: {}", fs_world::economy::health(w));
        println!(
            "  Market: {}",
            fs_world::transfer::MarketSummary::of(w, summary.year)
        );
        cups_line(w, summary.year, human);
        let mood = fs_world::happiness::Mood::of(w);
        let requests = w.relations().requests_made();
        println!(
            "  People: happiness mean {:+.1}, very unhappy {:.1}%; {} transfer requests",
            mood.mean,
            mood.unhappy * 100.0,
            requests - requests_before
        );
        requests_before = requests;
        let reps: Vec<u16> = w.clubs().iter().map(|(_, c)| c.reputation).collect();
        println!(
            "  Boards: {sackings} managers sacked; club reputation mean {:.0}, max {}",
            reps.iter().map(|&r| f64::from(r)).sum::<f64>() / reps.len().max(1) as f64,
            reps.iter().max().unwrap_or(&0)
        );
        for (row, &p) in prospect_rows.iter_mut().zip(&prospects) {
            row.push(ps.ability(p));
        }
        println!();
    }
    let w = game.world();
    println!("Prospects (ability by season, 0-1000; potential in brackets):");
    for (row, &p) in prospect_rows.iter().zip(&prospects) {
        let ps = w.players();
        let status = if ps.status(p) == Status::Retired {
            " retired"
        } else {
            ""
        };
        let abil: Vec<String> = row.iter().map(u16::to_string).collect();
        println!(
            "  {:<26} ({}) {}{status}",
            w.player_name(p),
            ps.potential(p),
            abil.join(" ")
        );
    }
}

/// Cup winners of a finished season: the followed club's national cup
/// and every continental cup, with how many cups found a winner.
fn cups_line(w: &fs_world::World, year: i32, human: Option<ClubId>) {
    let won = |c: &fs_world::Cup| {
        c.history()
            .iter()
            .rev()
            .find(|s| s.year == year)
            .and_then(|s| s.winner)
    };
    let total = w.cups().len();
    let decided = w.cups().iter().filter(|(_, c)| won(c).is_some()).count();
    let mut parts = vec![format!("{decided}/{total} cups won")];
    let nation = human.map(|c| w.clubs()[c].nation);
    for (_, c) in w.cups().iter() {
        let show = c.is_continental() || Some(c.scope) == nation.map(fs_world::CupScope::Nation);
        if !show {
            continue;
        }
        if let Some(winner) = won(c) {
            let club = &w.clubs()[winner];
            parts.push(format!(
                "{}: {} ({})",
                c.format.short_name,
                club.name,
                w.nations()[club.nation].code
            ));
        }
    }
    println!("  Cups: {}", parts.join("; "));
}

/// Wage bills and money by reputation band, at the start and after
/// `--seasons` seasons (default none), for tuning the economy model.
fn economy_cmd(a: &Args) {
    use fs_world::economy::{player_wage_bill, revenue_potential};
    let mut game = Game::new(generate(&config(a)));
    let report = |w: &World| {
        let mut rows: Vec<(u16, f64, f64, f64, f64)> = w
            .clubs()
            .iter()
            .map(|(id, c)| {
                let f = c.finances();
                let ledger = f.history.last().map_or(f.season, |x| x.ledger);
                let pot = revenue_potential(c.reputation) as f64;
                let income = ledger.operating_income().max(1) as f64;
                (
                    c.reputation,
                    player_wage_bill(w, id) as f64 * 52.0 / pot,
                    ledger.player_wages as f64 / income,
                    income / pot,
                    f.balance as f64 / pot,
                )
            })
            .collect();
        rows.sort_by_key(|r| r.0);
        println!(
            "reputation    clubs  bill/potential  wages/income  income/potential  balance/potential  in debt"
        );
        for band in rows.chunks(rows.len().div_ceil(8).max(1)) {
            let n = band.len() as f64;
            let mean =
                |k: fn(&(u16, f64, f64, f64, f64)) -> f64| band.iter().map(k).sum::<f64>() / n;
            let debt = band.iter().filter(|r| r.4 < 0.0).count();
            println!(
                "{:>5}-{:<5}  {:>5}  {:>14.2}  {:>12.2}  {:>16.2}  {:>17.2}  {:>7}",
                band[0].0,
                band[band.len() - 1].0,
                band.len(),
                mean(|r| r.1),
                mean(|r| r.2),
                mean(|r| r.3),
                mean(|r| r.4),
                debt
            );
        }
        println!("{}\n", fs_world::economy::health(w));
    };
    report(game.world());
    for _ in 0..a.seasons.min(50) {
        let year = fs_world::people::season_year(game.world().date());
        let target = fs_core::GameDate::from_ymd(year + 1, 7, 2).expect("valid date");
        while game.world().date() < target {
            let days = target.days_since(game.world().date()).min(366) as u16;
            game.submit(Actor::System, Command::Advance { days })
                .expect("advance");
        }
    }
    if a.seasons > 0 {
        println!("After {} seasons:", a.seasons);
        report(game.world());
    }
}
