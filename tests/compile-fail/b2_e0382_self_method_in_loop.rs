//~ ERROR E0382
struct Builder { parts: Vec<String> }
impl Builder {
    fn add(mut self, p: &str) -> Builder { self.parts.push(p.to_string()); self }
}
fn main() {
    let b = Builder { parts: vec![] };
    for p in ["a", "b"] {
        let _ = b.add(p);
    }
}
