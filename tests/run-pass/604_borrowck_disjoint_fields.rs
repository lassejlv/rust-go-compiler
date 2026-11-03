// Disjoint field borrows of the same struct.
struct S {
    x: Vec<i32>,
    y: Vec<i32>,
    name: String,
}

fn main() {
    let mut s = S { x: vec![1, 2], y: vec![10], name: String::from("s") };
    let a = &mut s.x;
    let b = &mut s.y;
    a.push(3);
    b.push(a.len() as i32);
    let n = &s.name;
    s.x.push(n.len() as i32);
    for v in s.x.iter_mut() {
        s.y.push(*v);
        *v *= 2;
    }
    let S { x, y, name } = &mut s;
    x.push(y.len() as i32);
    name.push('!');
    println!("{:?} {:?} {}", s.x, s.y, s.name);
}
