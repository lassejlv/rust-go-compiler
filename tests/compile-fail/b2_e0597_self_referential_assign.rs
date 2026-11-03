//~ ERROR E0597
struct Node<'a> { next: Option<&'a Node<'a>>, v: i32 }
fn main() {
    let mut head = Node { next: None, v: 0 };
    {
        let tail = Node { next: None, v: 1 };
        head.next = Some(&tail);
    }
    println!("{}", head.v);
}
