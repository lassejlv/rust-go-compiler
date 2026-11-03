//~ ERROR lifetime
trait Get<'a> { fn get(&self) -> &'a str; }
struct H { s: String }
impl<'a> Get<'a> for H {
    fn get(&self) -> &'a str { &self.s }
}
fn main() { let h = H { s: "x".into() }; println!("{}", h.get()); }
