//~ ERROR E0599
// A method of a generic impl is checked against the impl's bounds (T has
// no `len`), even though the only instance (T = String) has one.
struct Wrap<T>(T);

impl<T> Wrap<T> {
    fn size(&self) -> usize {
        self.0.len()
    }
}

fn main() {
    println!("{}", Wrap(String::from("abc")).size());
}
