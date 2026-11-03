//~ ERROR E0507
struct S { inner: Option<Vec<u8>> }
fn f(s: &S) -> Vec<u8> {
    if let Some(v) = s.inner { v } else { Vec::new() }
}
fn main() { f(&S { inner: None }); }
