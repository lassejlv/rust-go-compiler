//~ ERROR E0382
struct Builder { parts: Vec<String> }
impl Builder {
    fn add(mut self, p: &str) -> Self { self.parts.push(p.into()); self }
    fn build(self) -> String { self.parts.join(",") }
}
fn main() {
    let b = Builder { parts: vec![] };
    b.add("a");
    println!("{}", b.build());
}
