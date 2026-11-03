//~ ERROR E0716
fn main() {
    let words: Vec<&str>;
    words = String::from(" x y ").trim().split(' ').collect();
    println!("{:?}", words);
}
