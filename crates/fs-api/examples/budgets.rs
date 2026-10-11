//! Measures the performance budgets in `docs/ARCHITECTURE.md` section 6
//! on this machine: generation, memory, quiet days, match days, a season,
//! player search and saving.
//!
//! ```text
//! cargo run --release -p fs-api --example budgets [tiny|small|medium|large] [--season]
//! ```
//!
//! Set `RAYON_NUM_THREADS=1` to see what a single thread (the browser
//! build) manages.

use fs_api::dto::PlayerSearch;
use fs_api::query;
use fs_gen::{GenConfig, Size, generate};
use fs_world::{Actor, Command, Game};
use std::time::Instant;

/// Resident memory in bytes (Linux), if known.
fn rss() -> Option<u64> {
    let status = std::fs::read_to_string("/proc/self/status").ok()?;
    let line = status.lines().find(|l| l.starts_with("VmRSS:"))?;
    let kb: u64 = line.split_whitespace().nth(1)?.parse().ok()?;
    Some(kb * 1024)
}

fn ms(t: Instant) -> f64 {
    t.elapsed().as_secs_f64() * 1000.0
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let size = match args.first().map(String::as_str) {
        Some("tiny") => Size::Tiny,
        Some("small") => Size::Small,
        Some("large") => Size::Large,
        _ => Size::Medium,
    };
    let season = args.iter().any(|a| a == "--season");
    let before = rss();
    let t = Instant::now();
    let world = generate(&GenConfig::preset(size, 1));
    println!(
        "world          {size:?}: {} players, {} clubs, {} leagues, {} cups",
        world.players().len(),
        world.clubs().len(),
        world.leagues().len(),
        world.cups().len()
    );
    println!("generate       {:.0} ms", ms(t));
    if let (Some(a), Some(b)) = (before, rss()) {
        println!(
            "memory         {:.0} MB resident after generation",
            (b - a) as f64 / 1e6
        );
    }
    let t = Instant::now();
    let snapshot = world.to_snapshot();
    println!(
        "snapshot       {:.0} ms, {:.1} MB",
        ms(t),
        snapshot.len() as f64 / 1e6
    );

    // Player search over every player, as the transfers screen runs it.
    let viewer = world
        .human_managers()
        .next()
        .and_then(|m| world.managers()[m].club);
    let searches = [
        PlayerSearch::default(),
        PlayerSearch {
            positions: vec!["ST".into()],
            max_age: Some(24),
            ..PlayerSearch::default()
        },
        PlayerSearch {
            name: Some("an".into()),
            ..PlayerSearch::default()
        },
    ];
    for (k, s) in searches.iter().enumerate() {
        let t = Instant::now();
        let view = query::market::search(&world, viewer, s);
        println!("search {k}       {:.1} ms ({} matches)", ms(t), view.total);
    }

    // Days: quiet days and match days, measured separately.
    let mut game = Game::new(world);
    let days = if season { 365 } else { 60 };
    let (mut quiet, mut busy) = (Vec::new(), Vec::new());
    let start = Instant::now();
    for _ in 0..days {
        let t = Instant::now();
        game.submit(Actor::System, Command::Advance { days: 1 })
            .expect("advance");
        let w = game.world();
        let today = w.date();
        let matches = w.leagues().iter().any(|(_, l)| {
            l.season()
                .is_some_and(|s| s.fixtures.iter().any(|f| f.date == today))
        }) || w.cups().iter().any(|(_, c)| {
            c.edition()
                .is_some_and(|e| e.fixtures.iter().any(|f| f.fixture.date == today))
        });
        if matches {
            busy.push(ms(t));
        } else {
            quiet.push(ms(t));
        }
    }
    let total = start.elapsed().as_secs_f64();
    let stats = |v: &mut Vec<f64>| {
        v.sort_by(f64::total_cmp);
        let mean = v.iter().sum::<f64>() / v.len().max(1) as f64;
        let p95 = v.get(v.len() * 95 / 100).copied().unwrap_or(0.0);
        (v.len(), mean, p95, v.last().copied().unwrap_or(0.0))
    };
    let (n, mean, p95, max) = stats(&mut quiet);
    println!("quiet days     {n}: mean {mean:.1} ms, 95th {p95:.1} ms, worst {max:.0} ms");
    let (n, mean, p95, max) = stats(&mut busy);
    println!("match days     {n}: mean {mean:.1} ms, 95th {p95:.1} ms, worst {max:.0} ms");
    if season {
        println!("season         {total:.1} s");
    } else {
        println!(
            "{days} days        {total:.1} s (about {:.0} s a season)",
            total * 365.0 / f64::from(days)
        );
    }
    if let Some(b) = rss() {
        println!(
            "memory         {:.0} MB resident at the end",
            b as f64 / 1e6
        );
    }
}
