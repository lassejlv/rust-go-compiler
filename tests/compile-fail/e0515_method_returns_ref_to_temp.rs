//~ ERROR E0515
struct Name {
    first: String,
    last: String,
}

impl Name {
    fn full(&self) -> &str {
        &format!("{} {}", self.first, self.last)
    }
}

fn main() {
    let n = Name { first: "a".into(), last: "b".into() };
    println!("{}", n.full());
}
