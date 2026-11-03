//~ ERROR E0382
fn main() {
    let t = (String::from("a"), String::from("b"));
    let a = t.0;
    let whole = t;
    println!("{} {:?}", a, whole);
}
