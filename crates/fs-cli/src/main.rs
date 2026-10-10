//! `fs-cli`: headless runner for footballsim.
//!
//! ```text
//! fs-cli gen         [--size tiny|small|medium|large] [--seed N]
//! fs-cli run         [--size ...] [--seed N] [--days N]
//! fs-cli squad       [--size ...] [--seed N] [--club N]
//! fs-cli determinism [--size ...] [--seed N] [--days N]
//! ```
//!
//! Builds for native and `wasm32-wasip1`; `scripts/determinism-check.sh`
//! runs `determinism` on both and compares the output.

use fs_core::state_hash;
use fs_gen::{GenConfig, Size, generate, scenario};
use fs_world::attributes::{Attr, Position};
use fs_world::{Actor, ClubId, Command, Game};
use std::process::ExitCode;
use std::time::Instant;

struct Args {
    size: Size,
    seed: u64,
    days: u16,
    club: u32,
}

const USAGE: &str = "usage: fs-cli <gen|run|squad|determinism> [--size tiny|small|medium|large] [--seed N] [--days N] [--club N]";

fn parse(mut args: impl Iterator<Item = String>) -> Result<(String, Args), String> {
    let cmd = args.next().ok_or_else(|| USAGE.to_string())?;
    let mut out = Args {
        size: Size::Tiny,
        seed: 1,
        days: 365,
        club: 0,
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
        _ => {
            eprintln!("{USAGE}");
            return ExitCode::from(2);
        }
    }
    ExitCode::SUCCESS
}

fn ms(t: Instant) -> f64 {
    t.elapsed().as_secs_f64() * 1000.0
}

fn gen_cmd(a: &Args) {
    let t = Instant::now();
    let world = generate(&GenConfig::preset(a.size, a.seed));
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
    let world = generate(&GenConfig::preset(a.size, a.seed));
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
    let world = generate(&GenConfig::preset(a.size, a.seed));
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
}
