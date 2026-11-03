/// ```
/// assert_eq!(testpkg::text::shout("a"), "A!");
/// ```
pub fn shout(s: &str) -> String { s.to_uppercase() + "!" }

#[cfg(test)]
mod tests {
    #[test]
    fn shouts() { assert_eq!(super::shout("hi"), "HI!"); }
}
