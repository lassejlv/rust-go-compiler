// NLL: a shared borrow ends at its last use, so later mutation is fine.
fn main() {
    let mut v = vec![1, 2, 3];
    let first = &v[0];
    println!("first = {}", first);
    v.push(4);
    let r = &mut v;
    r.push(5);
    println!("{:?}", v);
    let mut s = String::from("ab");
    let len = s.len();
    let view = &s;
    let up = view.to_uppercase();
    s.push_str(&up);
    println!("{} {}", s, len);
}
