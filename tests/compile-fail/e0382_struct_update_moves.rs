//~ ERROR E0382
#[derive(Debug)]
struct Config {
    name: String,
    level: u8,
}

fn main() {
    let base = Config { name: "base".to_string(), level: 1 };
    let derived = Config { level: 2, ..base };
    println!("{:?} {:?}", derived, base);
}
