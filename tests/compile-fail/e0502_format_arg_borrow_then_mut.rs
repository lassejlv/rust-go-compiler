//~ ERROR E0502
// A shared borrow held by an earlier format argument (`w.get(0)` returns a
// reference into `w`) is still alive when a later argument borrows `w`
// mutably: `new_debug(&x) -> __Arg<'a>` keeps what `x` points to alive.
struct W {
    v: Vec<i32>,
}
impl W {
    fn get(&self, i: usize) -> Option<&i32> {
        self.v.get(i)
    }
    fn pop(&mut self) -> bool {
        self.v.pop().is_some()
    }
}
fn main() {
    let mut w = W { v: vec![1, 2] };
    println!("{:?} {}", w.get(0), w.pop());
}
