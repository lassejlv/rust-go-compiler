//~ ERROR E0594
fn main() {
    let t = (1, String::new());
    t.0 = 5;
    println!("{:?}", t);
}
