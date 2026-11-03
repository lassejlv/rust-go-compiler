//~ ERROR E0382
fn main() {
    let s = String::from("dup");
    let v = vec![s, s];
    println!("{:?}", v);
}
