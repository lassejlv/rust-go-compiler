//~ ERROR E0499
fn swap_vals<T>(a: &mut T, b: &mut T) { std::mem::swap(a, b); }
fn main() {
    let mut arr = [1, 2];
    swap_vals(&mut arr[0], &mut arr[1]);
}
