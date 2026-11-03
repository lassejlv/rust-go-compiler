//~ ERROR E0506
struct Holder<'a> { r: &'a i32 }
fn main() {
    let mut n = 1;
    let h = Holder { r: &n };
    n = 2;
    println!("{} {}", h.r, n);
}
