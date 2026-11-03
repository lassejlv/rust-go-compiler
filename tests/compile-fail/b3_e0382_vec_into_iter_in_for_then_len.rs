//~ ERROR E0382
fn main() {
    let v = vec![String::from("a")];
    for s in v { println!("{}", s); }
    println!("{}", v.len());
}
