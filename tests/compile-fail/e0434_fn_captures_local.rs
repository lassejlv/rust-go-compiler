//~ ERROR E0434
fn main() {
    let y = 5;
    fn inner() -> i32 {
        y + 1
    }
    println!("{}", inner());
}
