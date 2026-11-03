//~ ERROR E0308
fn main() {
    let f = |x: i32| { if x > 0 { return "pos"; } 0 };
    f(1);
}
