// Calling &mut self methods that use results of &self methods.
struct Inventory {
    items: Vec<(String, u32)>,
}

impl Inventory {
    fn index_of(&self, name: &str) -> Option<usize> {
        self.items.iter().position(|(n, _)| n == name)
    }
    fn count(&self) -> u32 {
        self.items.iter().map(|(_, c)| c).sum()
    }
    fn add(&mut self, name: &str, c: u32) {
        match self.index_of(name) {
            Some(i) => self.items[i].1 += c,
            None => self.items.push((name.to_string(), c)),
        }
    }
    fn add_total(&mut self) {
        let t = self.count();
        self.add("total", t);
    }
    fn names(&self) -> Vec<&str> {
        self.items.iter().map(|(n, _)| n.as_str()).collect()
    }
}

fn main() {
    let mut inv = Inventory { items: Vec::new() };
    inv.add("apple", 3);
    inv.add("pear", 2);
    inv.add("apple", 1);
    inv.add_total();
    let c = inv.count();
    inv.add("pear", c);
    println!("{:?} {:?}", inv.names(), inv.items);
}
