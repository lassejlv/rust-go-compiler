//~ ERROR E0579
fn main() {
    let x = 7i32;
    match x {
        5..5 => println!("five"),
        _ => println!("other"),
    }
}
