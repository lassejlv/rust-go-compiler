//~ ERROR E0500
struct Counter { items: Vec<i32>, total: i32 }
impl Counter {
    fn bump(&mut self, x: i32) { self.total += x; }
    fn run(&mut self) {
        self.items.iter().for_each(|x| self.bump(*x));
    }
}
fn main() { let mut c = Counter { items: vec![1], total: 0 }; c.run(); }
