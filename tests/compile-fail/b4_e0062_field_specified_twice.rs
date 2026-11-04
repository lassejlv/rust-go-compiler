//~ ERROR E0062
struct Config { verbose: bool, level: u8 }
fn main() {
    let c = Config { verbose: true, level: 1, verbose: false };
    println!("{}", c.level);
}
