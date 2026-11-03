//~ ERROR E0381
fn main() {
    let x: i32;
    if std::env::args().count() > 3 {
        x = 1;
    }
    println!("{}", x);
}
