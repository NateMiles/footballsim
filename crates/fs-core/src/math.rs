//! Portable maths for simulation code.
//!
//! The standard library's `sin`, `exp`, `ln`, `powf` and friends call into
//! the platform's maths library, whose results can differ in the last bit
//! between targets. Simulation code must call these wrappers instead; they
//! route through the pure-Rust `libm` crate and give identical bits on
//! native and WebAssembly. Clippy enforces this (see `clippy.toml`).
//!
//! `sqrt`, `abs`, `floor`, `ceil`, `round`, `min` and `max` are exact under
//! IEEE 754 everywhere, so they are re-exported for convenience only.

macro_rules! wrap {
    ($($name:ident => $f64:ident, $f32:ident;)*) => {
        $(
            #[doc = concat!("Portable `", stringify!($name), "` for `f64`.")]
            #[inline]
            pub fn $name(x: f64) -> f64 { libm::$f64(x) }
        )*
        /// `f32` versions of the portable maths functions.
        pub mod f32 {
            $(
                #[doc = concat!("Portable `", stringify!($name), "` for `f32`.")]
                #[inline]
                pub fn $name(x: f32) -> f32 { libm::$f32(x) }
            )*

            /// Portable `atan2` for `f32`.
            #[inline]
            pub fn atan2(y: f32, x: f32) -> f32 { libm::atan2f(y, x) }

            /// Portable `powf` for `f32`.
            #[inline]
            pub fn powf(x: f32, y: f32) -> f32 { libm::powf(x, y) }

            /// Portable `hypot` for `f32`.
            #[inline]
            pub fn hypot(x: f32, y: f32) -> f32 { libm::hypotf(x, y) }
        }
    };
}

wrap! {
    sin => sin, sinf;
    cos => cos, cosf;
    tan => tan, tanf;
    asin => asin, asinf;
    acos => acos, acosf;
    atan => atan, atanf;
    exp => exp, expf;
    ln => log, logf;
    log2 => log2, log2f;
    log10 => log10, log10f;
    sqrt => sqrt, sqrtf;
    tanh => tanh, tanhf;
}

/// Portable `atan2` for `f64`.
#[inline]
pub fn atan2(y: f64, x: f64) -> f64 {
    libm::atan2(y, x)
}

/// Portable `powf` for `f64`.
#[inline]
pub fn powf(x: f64, y: f64) -> f64 {
    libm::pow(x, y)
}

/// Portable `hypot` for `f64`.
#[inline]
pub fn hypot(x: f64, y: f64) -> f64 {
    libm::hypot(x, y)
}

/// Logistic function `1 / (1 + e^-x)`, used throughout for attribute
/// contests.
#[inline]
pub fn logistic(x: f64) -> f64 {
    1.0 / (1.0 + exp(-x))
}

/// Linear interpolation between `a` and `b`.
#[inline]
pub fn lerp(a: f64, b: f64, t: f64) -> f64 {
    a + (b - a) * t
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn values_are_sane() {
        assert!((sin(core::f64::consts::FRAC_PI_2) - 1.0).abs() < 1e-15);
        assert!((ln(core::f64::consts::E) - 1.0).abs() < 1e-15);
        assert!((powf(2.0, 10.0) - 1024.0).abs() < 1e-12);
        assert!((logistic(0.0) - 0.5).abs() < 1e-15);
        assert!((f32::atan2(1.0, 1.0) - core::f32::consts::FRAC_PI_4).abs() < 1e-6);
    }

    /// Bit patterns pinned so the native-vs-WASM CI check has a cheap
    /// first line of defence.
    #[test]
    fn bit_exact_golden_values() {
        assert_eq!(exp(1.5).to_bits(), 0x4011_ed3f_e64f_c541);
        assert_eq!(ln(10.0).to_bits(), 0x4002_6bb1_bbb5_5516);
    }
}
