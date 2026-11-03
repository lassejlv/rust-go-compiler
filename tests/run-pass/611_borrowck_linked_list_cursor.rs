// Walking a linked list with a reassigned &mut cursor.
#[derive(Debug)]
struct Node {
    val: i32,
    next: Option<Box<Node>>,
}

fn push_back(head: &mut Option<Box<Node>>, val: i32) {
    let mut cur = head;
    while let Some(node) = cur {
        cur = &mut node.next;
    }
    *cur = Some(Box::new(Node { val, next: None }));
}

fn add_all(head: &mut Option<Box<Node>>, k: i32) {
    let mut cur = head.as_mut();
    while let Some(node) = cur {
        node.val += k;
        cur = node.next.as_mut();
    }
}

fn to_vec(head: &Option<Box<Node>>) -> Vec<i32> {
    let mut out = Vec::new();
    let mut cur = head;
    while let Some(n) = cur {
        out.push(n.val);
        cur = &n.next;
    }
    out
}

fn main() {
    let mut list = None;
    for i in 1..=4 {
        push_back(&mut list, i);
    }
    add_all(&mut list, 10);
    println!("{:?}", to_vec(&list));
}
