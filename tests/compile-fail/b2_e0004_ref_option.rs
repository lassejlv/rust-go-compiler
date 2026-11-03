//~ ERROR E0004
fn f(o: &Option<String>) -> usize {
    match o {
        Some(s) if s.len() > 2 => 1,
        None => 0,
    }
}
fn main() { f(&None); }
