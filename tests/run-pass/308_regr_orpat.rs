fn f(input: &[u8]) -> usize {
    match (input.len(), input) {
        (3, s) | (5, [b'{', s @ .., b'}']) => s.len() * 100 + s[0] as usize,
        _ => 0,
    }
}
fn main() { println!("{} {} {}", f(b"abc"), f(b"{abc}"), f(b"xy")); }
