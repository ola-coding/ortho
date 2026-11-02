//! Numbers as JavaScript writes them, which is what the SVGs have always held.
use std::fmt;

/// A number as a JavaScript string: the shortest digits that read back the same, the
/// same as Rust's, but negative zero is `0`, and very large and very small numbers
/// take an exponent.
pub struct Js(pub f64);

impl fmt::Display for Js {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let x = self.0;
        if x == 0.0 {
            f.write_str("0")
        } else if x.is_nan() {
            f.write_str("NaN")
        } else if x.is_infinite() {
            f.write_str(if x > 0.0 { "Infinity" } else { "-Infinity" })
        } else if (1e-6..1e21).contains(&x.abs()) {
            write!(f, "{x}")
        } else {
            let written = format!("{x:e}");
            match written.split_once('e') {
                Some((digits, exponent)) if !exponent.starts_with('-') => write!(f, "{digits}e+{exponent}"),
                _ => f.write_str(&written),
            }
        }
    }
}

/// `Math.hypot` of two numbers, as V8 computes it. A platform's own can differ in the
/// last digit, which a label placed along a line would show.
pub fn hypot(x: f64, y: f64) -> f64 {
    let (x, y) = (x.abs(), y.abs());
    let max = x.max(y);
    if max == 0.0 {
        return 0.0;
    }
    let (mut sum, mut compensation) = (0.0, 0.0);
    for value in [x, y] {
        let n = value / max;
        let summand = n * n - compensation;
        let preliminary = sum + summand;
        compensation = (preliminary - sum) - summand;
        sum = preliminary;
    }
    sum.sqrt() * max
}
