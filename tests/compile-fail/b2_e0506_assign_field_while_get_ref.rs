//~ ERROR E0506
struct Cache { val: String }
impl Cache { fn get(&self) -> &str { &self.val } }
fn main() {
    let mut c = Cache { val: "a".into() };
    let r = c.get();
    c.val = String::from("b");
    println!("{}", r);
}
