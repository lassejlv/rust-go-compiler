//~ ERROR E0502
fn main() {
    let mut s = vec![String::from("a")];
    let f = s.first();
    s.clear();
    println!("{:?}", f);
}
