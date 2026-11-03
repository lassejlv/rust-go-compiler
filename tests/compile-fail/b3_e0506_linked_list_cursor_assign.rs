//~ ERROR E0506
struct Node { val: i32, next: Option<Box<Node>> }
fn main() {
    let mut head = Node { val: 1, next: None };
    let r = &head.val;
    head = Node { val: 2, next: None };
    println!("{} {:?}", r, head.next.is_none());
}
