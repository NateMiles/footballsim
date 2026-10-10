use criterion::{Criterion, black_box, criterion_group, criterion_main};
use fs_core::{Rng, Seed, Stream};

fn bench(c: &mut Criterion) {
    let mut rng = Rng::from_u64(1);
    c.bench_function("rng/next_u64", |b| b.iter(|| black_box(rng.next_u64())));
    c.bench_function("rng/below_23", |b| b.iter(|| black_box(rng.below(23))));
    c.bench_function("rng/normal", |b| b.iter(|| black_box(rng.normal(0.0, 1.0))));
    let seed = Seed(5);
    let mut k = 0u64;
    c.bench_function("rng/derive", |b| {
        b.iter(|| {
            k += 1;
            black_box(seed.derive(Stream::DailyTick, k, 3))
        })
    });
}

criterion_group!(benches, bench);
criterion_main!(benches);
