//~ ERROR E0382
struct Config { name: String, level: u8 }
impl Config {
    fn finish(self) -> String { self.name }
}
fn main() {
    let c = Config { name: "x".into(), level: 3 };
    let n = c.finish();
    println!("{} {}", n, c.level);
}
