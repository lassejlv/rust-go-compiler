//~ ERROR E0502
struct Config { name: String, retries: u32 }
impl Config {
    fn name(&self) -> &str { &self.name }
    fn set_retries(&mut self, r: u32) { self.retries = r; }
}
fn main() {
    let mut c = Config { name: "x".into(), retries: 0 };
    let n = c.name();
    c.set_retries(3);
    println!("{}", n);
}
