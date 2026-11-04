//~ ERROR E0502
// `&v.items` as a format argument keeps `v` borrowed while `v.clear()` runs.
struct V {
    items: Vec<u8>,
}
impl V {
    fn clear(&mut self) -> usize {
        let n = self.items.len();
        self.items.clear();
        n
    }
}
fn main() {
    let mut v = V { items: vec![1, 2, 3] };
    let s = format!("{:?} {}", &v.items, v.clear());
    println!("{}", s);
}
