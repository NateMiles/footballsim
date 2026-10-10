//! Data-parallel helpers that fall back to sequential code on WebAssembly.
//!
//! Work items must be independent and derive their own RNG streams, so the
//! result is identical whether this runs on one thread or many.

/// Calls `f(index, item)` for every element of `items`, in parallel on
/// native targets. Tuned for many cheap items (per-player updates): work
/// is split into batches of at least 1024.
pub fn for_each_indexed_mut<T, F>(items: &mut [T], f: F)
where
    T: Send,
    F: Fn(usize, &mut T) + Sync + Send,
{
    for_each_indexed_mut_batched(items, 1024, f);
}

/// Like [`for_each_indexed_mut`], for expensive items such as whole
/// matches: every item may run on its own thread.
pub fn for_each_heavy<T, F>(items: &mut [T], f: F)
where
    T: Send,
    F: Fn(usize, &mut T) + Sync + Send,
{
    for_each_indexed_mut_batched(items, 1, f);
}

fn for_each_indexed_mut_batched<T, F>(items: &mut [T], min_batch: usize, f: F)
where
    T: Send,
    F: Fn(usize, &mut T) + Sync + Send,
{
    #[cfg(not(target_family = "wasm"))]
    {
        use rayon::prelude::*;
        items
            .par_iter_mut()
            .with_min_len(min_batch)
            .enumerate()
            .for_each(|(i, t)| f(i, t));
    }
    #[cfg(target_family = "wasm")]
    {
        let _ = min_batch;
        items.iter_mut().enumerate().for_each(|(i, t)| f(i, t));
    }
}
