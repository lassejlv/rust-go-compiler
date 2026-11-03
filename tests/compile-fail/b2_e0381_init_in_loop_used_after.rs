//~ ERROR E0381
fn main() {
    let mut x: i32;
    for i in 0..3 {
        x = i;
        println!("{}", x);
    }
    println!("{}", x);
}
