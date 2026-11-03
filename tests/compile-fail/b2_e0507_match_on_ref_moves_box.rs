//~ ERROR E0507
enum Tree { Leaf(i32), Node(Box<Tree>) }
fn depth(t: &Tree) -> i32 {
    match *t {
        Tree::Leaf(_) => 0,
        Tree::Node(inner) => 1 + depth(&inner),
    }
}
fn main() { println!("{}", depth(&Tree::Node(Box::new(Tree::Leaf(1))))); }
