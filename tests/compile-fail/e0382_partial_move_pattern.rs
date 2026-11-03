//~ ERROR E0382
struct Pair {
    a: String,
    b: String,
}

fn main() {
    let p = Pair { a: "x".to_string(), b: "y".to_string() };
    let Pair { a, .. } = p;
    println!("{}", a);
    let q = p;
    println!("{}", q.b);
}
