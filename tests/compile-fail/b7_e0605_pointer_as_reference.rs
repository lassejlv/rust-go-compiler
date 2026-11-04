//~ ERROR E0605
// A raw pointer cannot be cast to a reference (dereference it instead).
fn main() {
    let v = 3;
    let p = &v as *const i32;
    let r = p as &i32;
    println!("{}", r);
}
