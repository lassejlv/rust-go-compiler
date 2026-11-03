//~ ERROR E0046
struct Counter(u32);
impl Iterator for Counter {
    type Item = u32;
    fn size_hint(&self) -> (usize, Option<usize>) { (0, None) }
}
fn main() {}
