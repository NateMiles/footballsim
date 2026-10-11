//! Match engine benchmarks. Targets: tier 1 under 1 ms, tier 2 under
//! 10 µs per match (docs/ARCHITECTURE.md, section 6).

use criterion::{Criterion, black_box, criterion_group, criterion_main};
use fs_core::Seed;
use fs_core::attributes::{ATTR_COUNT, Attributes, Hidden, PositionRatings};
use fs_core::ids::PlayerId;
use fs_match::{Formation, MatchInput, PlayerSnapshot, Tactic, TeamSheet, tier1, tier2};

fn sheet(level: u16, first: u32) -> TeamSheet {
    let tactic = Tactic::balanced(Formation::F4231);
    let slots = tactic.formation.slots();
    let make = |i: u32, pos| {
        let mut positions = PositionRatings::default();
        positions.set(pos, 20);
        let mut attributes = Attributes([level; ATTR_COUNT]);
        // A little variety so contests are not all identical.
        attributes
            .0
            .iter_mut()
            .enumerate()
            .for_each(|(k, v)| *v = (*v + (k as u16 * 37 + i as u16 * 11) % 120).min(1000));
        PlayerSnapshot {
            id: PlayerId::new(first + i),
            attributes,
            hidden: Hidden::default(),
            positions,
            fitness: 10_000,
            sharpness: 8_000,
            morale: 5_000,
            cohesion: fs_match::input::DRILLED,
        }
    };
    TeamSheet {
        tactic,
        starters: (0..11)
            .map(|i| make(i, slots[i as usize].position))
            .collect(),
        bench: (0..9)
            .map(|i| make(11 + i, slots[1 + i as usize].position))
            .collect(),
        auto_subs: true,
        instructions: Default::default(),
    }
}

fn bench(c: &mut Criterion) {
    let mut input = MatchInput::new(sheet(620, 0), sheet(560, 100), Seed(1), 0);
    c.bench_function("tier1/match", |b| {
        b.iter(|| {
            input.key += 1;
            black_box(tier1::play(&input))
        })
    });
    c.bench_function("tier2/match", |b| {
        b.iter(|| {
            input.key += 1;
            black_box(tier2::play(&input))
        })
    });
}

criterion_group!(benches, bench);
criterion_main!(benches);
