//~ ERROR E0382
fn main() {
    let v = vec![String::from("a"), String::from("b")];
    for s in v {
        println!("{}", s);
    }
    println!("{}", v.len());
}
