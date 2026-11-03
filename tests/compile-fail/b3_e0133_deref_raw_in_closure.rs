//~ ERROR E0133
fn main() {
    let x = 5;
    let p = &x as *const i32;
    let f = || *p + 1;
    println!("{}", f());
}
