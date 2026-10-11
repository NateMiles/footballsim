//! Optional timing of the daily tick's stages, for finding slow parts.
//!
//! Off unless the `profile` feature is on, when [`time`] adds each
//! stage's wall-clock time to a running total that [`report`] returns.
//! Timing never changes what the simulation does.

#[cfg(feature = "profile")]
mod enabled {
    use std::collections::BTreeMap;
    use std::sync::Mutex;
    use std::time::Instant;

    static TOTALS: Mutex<BTreeMap<&'static str, (f64, u64)>> = Mutex::new(BTreeMap::new());

    pub fn time<R>(stage: &'static str, f: impl FnOnce() -> R) -> R {
        let t = Instant::now();
        let r = f();
        let secs = t.elapsed().as_secs_f64();
        let mut totals = TOTALS.lock().expect("profile lock");
        let e = totals.entry(stage).or_default();
        e.0 += secs;
        e.1 += 1;
        r
    }

    pub fn report() -> Vec<(&'static str, f64, u64)> {
        let totals = TOTALS.lock().expect("profile lock");
        let mut v: Vec<_> = totals.iter().map(|(&k, &(s, n))| (k, s, n)).collect();
        v.sort_by(|a, b| b.1.total_cmp(&a.1));
        v
    }

    pub fn reset() {
        TOTALS.lock().expect("profile lock").clear();
    }
}

#[cfg(feature = "profile")]
pub use enabled::{report, reset, time};

/// Runs `f`, timing it under `stage` when profiling.
#[cfg(not(feature = "profile"))]
#[inline(always)]
pub fn time<R>(_stage: &'static str, f: impl FnOnce() -> R) -> R {
    f()
}

/// Stage totals: name, seconds and calls, slowest first. Empty unless
/// profiling.
#[cfg(not(feature = "profile"))]
pub fn report() -> Vec<(&'static str, f64, u64)> {
    Vec::new()
}

/// Clears the totals.
#[cfg(not(feature = "profile"))]
pub fn reset() {}
