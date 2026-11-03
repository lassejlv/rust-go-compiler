//~ ERROR E0133
unsafe fn danger() -> i32 {
    1
}

fn main() {
    println!("{}", danger());
}
