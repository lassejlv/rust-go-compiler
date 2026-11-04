//~ ERROR E0496
// A method's lifetime parameter may not shadow the impl's.
struct Holder<'a>(&'a str);

impl<'a> Holder<'a> {
    fn pick<'a>(&self, other: &'a str) -> &'a str {
        other
    }
}

fn main() {
    let h = Holder("a");
    println!("{}", h.pick("b"));
}
