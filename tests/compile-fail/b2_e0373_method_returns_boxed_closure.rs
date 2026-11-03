//~ ERROR E0373
struct C { k: i32 }
impl C {
    fn mk(&self) -> Box<dyn Fn() -> i32 + 'static> {
        let k = self.k;
        Box::new(|| k * 2)
    }
}
fn main() { println!("{}", C { k: 1 }.mk()()); }
