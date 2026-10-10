//! World-level benchmarks. Targets are in `docs/ARCHITECTURE.md`,
//! section 6.

use criterion::{BatchSize, Criterion, black_box, criterion_group, criterion_main};
use fs_gen::{GenConfig, Size, generate};
use fs_world::{Actor, Command, Game, World};
use std::time::Duration;

fn generation(c: &mut Criterion) {
    let mut g = c.benchmark_group("generate");
    g.sample_size(10).measurement_time(Duration::from_secs(5));
    for (name, size) in [("tiny", Size::Tiny), ("small", Size::Small)] {
        let cfg = GenConfig::preset(size, 1);
        g.bench_function(name, |b| b.iter(|| black_box(generate(&cfg))));
    }
    g.finish();
}

fn daily_tick(c: &mut Criterion) {
    let mut g = c.benchmark_group("daily_tick");
    g.sample_size(30);
    for (name, size) in [("small", Size::Small), ("medium", Size::Medium)] {
        let mut game = Game::new(generate(&GenConfig::preset(size, 1)));
        g.bench_function(name, |b| {
            b.iter(|| {
                game.submit(Actor::System, Command::Advance { days: 1 })
                    .expect("advance")
            })
        });
    }
    g.finish();
}

fn snapshot(c: &mut Criterion) {
    let mut g = c.benchmark_group("snapshot");
    g.sample_size(10);
    let world = generate(&GenConfig::preset(Size::Small, 1));
    let bytes = world.to_snapshot();
    g.bench_function("encode_small", |b| {
        b.iter(|| black_box(world.to_snapshot()))
    });
    g.bench_function("decode_small", |b| {
        b.iter_batched(
            || bytes.clone(),
            |bytes| black_box(World::from_snapshot(&bytes).expect("decode")),
            BatchSize::LargeInput,
        )
    });
    g.bench_function("hash_small", |b| b.iter(|| black_box(world.state_hash())));
    g.finish();
}

criterion_group!(benches, generation, daily_tick, snapshot);
criterion_main!(benches);
