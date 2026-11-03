//~ ERROR E0597
struct Holder<'a> { items: Vec<&'a str> }
fn main() {
    let mut h = Holder { items: Vec::new() };
    for i in 0..3 {
        let s = format!("{}", i);
        h.items.push(&s);
    }
    println!("{:?}", h.items);
}
