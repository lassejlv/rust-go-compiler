//~ ERROR E0382
fn consume(s: String) -> usize {
    s.len()
}

fn main() {
    let s = String::from("hello");
    let n = consume(s);
    println!("{} {}", n, s);
}
