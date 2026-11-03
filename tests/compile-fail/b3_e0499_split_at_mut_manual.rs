//~ ERROR E0499
fn halves(v: &mut [i32]) -> (&mut [i32], &mut [i32]) {
    let n = v.len() / 2;
    (&mut v[..n], &mut v[n..])
}
fn main() { let mut a = [1, 2, 3, 4]; let (x, y) = halves(&mut a); x[0] = y[0]; }
