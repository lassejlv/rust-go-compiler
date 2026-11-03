//~ ERROR lifetime may not live long enough
struct Store<'a> {
    items: Vec<&'a str>,
}

impl<'a> Store<'a> {
    fn add<'b>(&mut self, s: &'b str) {
        self.items.push(s);
    }
}

fn main() {
    let mut st = Store { items: vec![] };
    st.add("x");
    println!("{}", st.items.len());
}
