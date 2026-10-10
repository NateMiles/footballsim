//! Times the parts of a season in a medium world.
use fs_gen::{GenConfig, Size, generate};
use fs_world::{Actor, Command, Game};
use std::time::Instant;

fn main() {
    let t = Instant::now();
    let mut game = Game::new(generate(&GenConfig::preset(Size::Medium, 1)));
    println!("generate {:.2}s", t.elapsed().as_secs_f64());
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
}
