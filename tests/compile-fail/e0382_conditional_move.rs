//~ ERROR E0382
fn main() {
    let s = String::from("maybe");
    let c = std::env::args().count() > 5;
    if c {
        drop(s);
    }
    println!("{}", s);
}
