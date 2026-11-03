// Moving out of Box by deref, and mutating through Box.
#[derive(Debug)]
struct Big {
    v: Vec<i32>,
    s: String,
}

fn main() {
    let b = Box::new(Big { v: vec![1, 2], s: "s".into() });
    let Big { v, s } = *b;
    println!("{:?} {}", v, s);
    let mut bb = Box::new(Big { v: vec![], s: String::new() });
    bb.v.push(1);
    let vr = &mut bb.v;
    let sr = &mut bb.s;
    vr.push(2);
    sr.push('x');
    let inner = *bb;
    println!("{:?}", inner);
    let boxed_str = Box::new(String::from("moved"));
    let owned: String = *boxed_str;
    println!("{}", owned);
}
