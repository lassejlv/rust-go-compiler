//~ ERROR E0502
struct Stack { v: Vec<i32> }
impl Stack {
    fn peek(&self) -> Option<&i32> { self.v.last() }
    fn push(&mut self, x: i32) { self.v.push(x); }
}
fn main() {
    let mut s = Stack { v: vec![1] };
    if let Some(top) = s.peek() {
        s.push(*top + 1);
        println!("{}", top);
    }
}
