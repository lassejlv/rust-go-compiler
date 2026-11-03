//~ ERROR E0515
trait Named { fn name(&self) -> &str; }
struct U(u32);
impl Named for U {
    fn name(&self) -> &str {
        let s = self.0.to_string();
        &s
    }
}
fn main() { println!("{}", U(3).name()); }
