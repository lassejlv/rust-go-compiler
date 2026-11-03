//~ ERROR E0317
fn main() {
    let c = true;
    let x: i32 = if c { 5 };
    println!("{}", x);
}
