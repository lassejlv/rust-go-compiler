//~ ERROR E0499
fn swap_ends<T>(v: &mut Vec<T>) {
    let n = v.len();
    let a = &mut v[0];
    let b = &mut v[n - 1];
    std::mem::swap(a, b);
}
fn main() {}
