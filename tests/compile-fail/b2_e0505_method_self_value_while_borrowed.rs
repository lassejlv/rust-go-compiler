//~ ERROR E0505
struct Doc { body: String }
impl Doc { fn into_body(self) -> String { self.body } }
fn main() {
    let d = Doc { body: "b".into() };
    let r = &d.body;
    let b = d.into_body();
    println!("{} {}", r, b);
}
