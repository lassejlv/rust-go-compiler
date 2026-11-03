//~ ERROR E0382
fn main() {
    let o = Some(String::from("x"));
    let l = o.map(|s| s.len());
    println!("{:?} {:?}", l, o);
}
