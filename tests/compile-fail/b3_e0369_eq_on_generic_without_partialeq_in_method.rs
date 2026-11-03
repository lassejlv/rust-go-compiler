//~ ERROR E0369
struct Set<T> { v: Vec<T> }
impl<T> Set<T> {
    fn has(&self, t: &T) -> bool { self.v.iter().any(|x| x == t) }
}
fn main() { let s = Set { v: vec![1] }; s.has(&1); }
