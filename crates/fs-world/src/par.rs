//! Data-parallel helpers that fall back to sequential code on WebAssembly.
//!
//! Work items must be independent and derive their own RNG streams, so the
//! result is identical whether this runs on one thread or many.

/// Calls `f(index, item)` for every element of `items`, in parallel on
/// native targets.
pub fn for_each_indexed_mut<T, F>(items: &mut [T], f: F)
where
    T: Send,
    F: Fn(usize, &mut T) + Sync + Send,
{
    #[cfg(not(target_family = "wasm"))]
    {
        use rayon::prelude::*;
        items
            .par_iter_mut()
            .with_min_len(1024)
            .enumerate()
            .for_each(|(i, t)| f(i, t));
    }
    #[cfg(target_family = "wasm")]
    {
        items.iter_mut().enumerate().for_each(|(i, t)| f(i, t));
    }
}
