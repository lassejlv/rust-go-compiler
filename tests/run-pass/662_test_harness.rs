// Built with --test: libtest-compatible harness, output capture, should_panic,
// ignore, Result-returning tests, filters via program arguments.
pub fn add(a: i32, b: i32) -> i32 { a + b }

fn main() { println!("main must not run under --test"); }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adds() { println!("captured, not shown"); assert_eq!(add(2, 2), 4); }

    #[test]
    fn fails() { println!("shown on failure"); assert_eq!(add(2, 2), 5, "math"); }

    #[test]
    #[should_panic(expected = "boom")]
    fn panics_ok() { panic!("boom here"); }

    #[test]
    #[should_panic]
    fn no_panic() {}

    #[test]
    #[ignore = "slow"]
    fn ignored() {}

    #[test]
    fn result_ok() -> Result<(), String> { Ok(()) }

    #[test]
    fn result_err() -> Result<(), String> { Err("bad".to_string()) }
}

#[cfg(not(test))]
fn helper() {}
