//! Times the parts of a season in a medium world.
//!
//! With `--features fs-world/profile`, also breaks the time down by stage
//! of the daily tick.
use fs_gen::{GenConfig, Size, generate};
use fs_world::{Actor, Command, Game};
use std::time::Instant;

fn main() {
    let t = Instant::now();
    let size = match std::env::args().nth(1).as_deref() {
        Some("small") => Size::Small,
        Some("tiny") => Size::Tiny,
        Some("large") => Size::Large,
        _ => Size::Medium,
    };
    let mut game = Game::new(generate(&GenConfig::preset(size, 1)));
    println!("generate {:.2}s", t.elapsed().as_secs_f64());
    fs_world::profile::reset();
    let mut slow = Vec::new();
    for _ in 0..365 {
        let t = Instant::now();
        game.submit(Actor::System, Command::Advance { days: 1 })
            .unwrap();
        slow.push((t.elapsed().as_secs_f64(), game.world().date()));
    }
    let total: f64 = slow.iter().map(|s| s.0).sum();
    slow.sort_by(|a, b| b.0.total_cmp(&a.0));
    println!("season {total:.2}s; slowest days:");
    for (secs, d) in slow.iter().take(8) {
        println!("  {d} {secs:.3}s");
    }
    let quiet: Vec<f64> = slow.iter().filter(|s| s.0 < 0.05).map(|s| s.0).collect();
    println!(
        "quiet days: {} averaging {:.1} ms",
        quiet.len(),
        1000.0 * quiet.iter().sum::<f64>() / quiet.len().max(1) as f64
    );
    let stages = fs_world::profile::report();
    if !stages.is_empty() {
        println!("by stage:");
        for (name, secs, calls) in stages {
            println!("  {name:<20} {secs:>7.2}s  ({calls} calls)");
        }
    }
}
