//~ ERROR E0502
struct Log {
    lines: Vec<String>,
}

impl Log {
    fn add(&mut self, s: &str) {
        self.lines.push(s.to_string());
    }
}

fn main() {
    let mut l = Log { lines: vec!["a".into()] };
    let first = &l.lines[0];
    l.add(first);
}
