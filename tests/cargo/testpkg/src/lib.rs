//! A small library with unit tests.
//!
//! ```
//! assert_eq!(testpkg::fib(1), 1);
//! ```
pub mod text;

/// Fibonacci numbers.
///
/// ```
/// use testpkg::fib;
/// assert_eq!(fib(10), 55);
/// # assert_eq!(fib(0), 0);
/// ```
///
/// Hidden setup and `?`:
///
/// ```rust
/// # fn parse(s: &str) -> Result<u32, std::num::ParseIntError> { s.parse() }
/// let n = parse("12")?;
/// assert_eq!(testpkg::fib(n), 144);
/// # Ok::<(), std::num::ParseIntError>(())
/// ```
///
/// ```should_panic
/// let v: Vec<u64> = Vec::new();
/// v[testpkg::fib(3) as usize];
/// ```
///
/// ```no_run
/// loop { testpkg::fib(3); }
/// ```
///
/// ```ignore
/// this is not rust
/// ```
///
/// ```compile_fail
/// let x: u32 = "no";
/// ```
///
/// ```text
/// plain text
/// ```
pub fn fib(n: u32) -> u64 {
    let (mut a, mut b) = (0u64, 1u64);
    for _ in 0..n {
        let t = a + b;
        a = b;
        b = t;
    }
    a
}

fn private_helper(x: i32) -> i32 { x * 2 }

/// Doubles.
pub struct Doubler;

impl Doubler {
    /// ```
    /// fn main() {
    ///     println!("{}", testpkg::Doubler::apply(4));
    ///     assert_eq!(testpkg::Doubler::apply(4), 8);
    /// }
    /// ```
    pub fn apply(x: i32) -> i32 { doubled(x) }
}

pub fn doubled(x: i32) -> i32 { private_helper(x) }

/// Doubles via `$crate`.
///
/// ```
/// assert_eq!(testpkg::twice!(5), 10);
/// ```
#[macro_export]
macro_rules! twice {
    ($e:expr) => {
        $crate::doubled($e)
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fib_small() {
        assert_eq!(fib(10), 55);
    }

    #[test]
    fn dollar_crate_macro() {
        assert_eq!(twice!(4), 8);
    }

    #[test]
    fn private_access() {
        assert_eq!(private_helper(4), 8);
    }

    #[test]
    #[should_panic(expected = "overflow")]
    fn overflow_panics() {
        let _ = fib(200).checked_mul(2).expect("overflow");
        panic!("attempt to add with overflow");
    }

    #[test]
    fn dev_dependency() {
        assert_eq!(mathx::square(5), 25);
    }

    #[test]
    fn include_relative() {
        assert_eq!(include_str!("../data/words.txt").split_whitespace().count(), 3);
    }

    #[test]
    #[ignore]
    fn expensive() {}
}
