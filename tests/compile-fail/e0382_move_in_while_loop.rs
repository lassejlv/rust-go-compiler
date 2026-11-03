//~ ERROR E0382
fn main() {
    let s = String::from("x");
    let mut out = Vec::new();
    let mut i = 0;
    while i < 2 {
        out.push(s);
        i += 1;
    }
    println!("{}", out.len());
}
