//~ ERROR E0515
use std::cell::RefCell;
struct S { items: RefCell<Vec<String>> }
impl S {
    fn first(&self) -> &String { &self.items.borrow()[0] }
}
fn main() { let s = S { items: RefCell::new(vec!["a".into()]) }; s.first(); }
