//~ ERROR E0507
struct Q { items: Vec<u32> }
impl Q {
    fn take(&self) -> Vec<u32> { self.items }
}
fn main() { println!("{:?}", Q { items: vec![] }.take()); }
