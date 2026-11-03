//~ ERROR E0277
struct Bag { items: Vec<i32> }
fn main() {
    let b = Bag { items: vec![1] };
    for x in b { println!("{}", x); }
}
