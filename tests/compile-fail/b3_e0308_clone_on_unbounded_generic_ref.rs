//~ ERROR E0308
struct Stack<T> { items: Vec<T> }
impl<T> Stack<T> {
    fn peek_owned(&self) -> Option<T> { self.items.last().map(|x| x.clone()) }
}
fn main() { let s: Stack<i32> = Stack { items: vec![] }; s.peek_owned(); }
