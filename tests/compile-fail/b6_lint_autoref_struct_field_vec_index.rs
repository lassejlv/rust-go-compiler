//~ ERROR lint
// dangerous_implicit_autorefs: the field projection does not help, the
// Vec inside `*node` is still borrowed implicitly.
struct Node {
    children: Vec<usize>,
}

fn first_child(node: *const Node) -> usize {
    unsafe { (*node).children[0] }
}

fn main() {
    let n = Node { children: vec![4, 5] };
    println!("{}", first_child(&n));
}
