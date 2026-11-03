//~ ERROR E0384
fn main() {
    for i in 0..3 {
        i = i * 2;
        println!("{}", i);
    }
}
