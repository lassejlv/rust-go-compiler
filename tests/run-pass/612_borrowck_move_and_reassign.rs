// Moving out of a variable and then re-initializing it.
fn consume(s: String) -> usize {
    s.len()
}

fn main() {
    let mut s = String::new();
    s.push('a');
    drop(s);
    s = String::from("x");
    println!("{}", s);
    let mut total = 0;
    let mut buf = String::from("ab");
    for i in 0..3 {
        total += consume(buf);
        buf = format!("{}{}", i, "zz");
    }
    println!("{} {}", total, buf);
    let mut v = vec![1];
    let w = v;
    v = vec![2, 3];
    println!("{:?} {:?}", v, w);
}
