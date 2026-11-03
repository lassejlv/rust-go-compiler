//~ ERROR E0621
struct Store<'a> {
    items: Vec<&'a str>,
}

impl<'a> Store<'a> {
    fn add(&mut self, s: &str) {
        self.items.push(s);
    }
}

fn main() {
    let mut st = Store { items: vec![] };
    st.add("x");
    println!("{}", st.items.len());
}
