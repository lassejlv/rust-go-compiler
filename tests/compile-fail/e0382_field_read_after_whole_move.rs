//~ ERROR E0382
struct S {
    a: Vec<i32>,
    b: i32,
}

fn main() {
    let s = S { a: vec![], b: 1 };
    let t = s;
    println!("{}", s.b);
    println!("{}", t.a.len());
}
