//~ ERROR E0506
struct Doc { title: String }
impl Doc { fn title(&self) -> &str { &self.title } }
fn main() {
    let mut d = Doc { title: "t".into() };
    let t = d.title();
    d.title = String::from("u");
    println!("{}", t);
}
