//~ ERROR literal out of range
// The lint also applies to bodies of generic functions that are never
// instantiated.
fn fill<T: Clone>(v: &mut Vec<T>, x: T) -> u8 {
    v.push(x);
    let limit: u8 = 300;
    limit
}
fn main() {}
