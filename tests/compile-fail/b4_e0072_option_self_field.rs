//~ ERROR E0072
// A struct containing itself by value (through Option) has infinite size.
struct Node {
    value: i32,
    next: Option<Node>,
}
fn main() {
    let n = Node { value: 1, next: None };
    println!("{}", n.value);
}
