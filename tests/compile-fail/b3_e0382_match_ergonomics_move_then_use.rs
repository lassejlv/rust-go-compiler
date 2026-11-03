//~ ERROR E0382
fn main() {
    let pair = (String::from("a"), String::from("b"));
    let (a, _) = pair;
    println!("{} {:?}", a, pair);
}
