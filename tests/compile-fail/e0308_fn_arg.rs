//~ ERROR E0308
fn square(x: u32) -> u32 {
    x * x
}

fn main() {
    println!("{}", square(2.5));
}
