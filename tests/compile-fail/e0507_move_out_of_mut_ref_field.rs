//~ ERROR E0507
struct Node {
    next: Option<Box<Node>>,
    val: i32,
}

fn detach(n: &mut Node) -> Option<Box<Node>> {
    n.next
}

fn main() {
    let mut n = Node { next: None, val: 1 };
    println!("{} {}", detach(&mut n).is_none(), n.val);
}
