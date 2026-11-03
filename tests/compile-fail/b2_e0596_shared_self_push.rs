//~ ERROR E0596
struct Log { lines: Vec<String> }
impl Log {
    fn add(&self, l: &str) { self.lines.push(l.to_string()); }
}
fn main() { let l = Log { lines: vec![] }; l.add("x"); }
