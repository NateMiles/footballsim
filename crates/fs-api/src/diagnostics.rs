//! Facts about the machine running the core, for performance tests.

/// Memory the core is using, in bytes, where the platform says: the
/// WebAssembly linear memory in a browser, resident memory on Linux.
pub fn memory_bytes() -> Option<u64> {
    #[cfg(target_arch = "wasm32")]
    {
        Some(core::arch::wasm32::memory_size(0) as u64 * 65_536)
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let status = std::fs::read_to_string("/proc/self/status").ok()?;
        let line = status.lines().find(|l| l.starts_with("VmRSS:"))?;
        let kb: u64 = line.split_whitespace().nth(1)?.parse().ok()?;
        Some(kb * 1024)
    }
}

/// Threads the simulation can use: one in a browser.
pub fn threads() -> u32 {
    #[cfg(target_family = "wasm")]
    {
        1
    }
    #[cfg(not(target_family = "wasm"))]
    {
        std::thread::available_parallelism().map_or(1, |n| n.get() as u32)
    }
}
