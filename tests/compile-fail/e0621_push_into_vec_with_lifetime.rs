//~ ERROR E0621
fn add<'a>(v: &mut Vec<&'a str>, s: &str) {
    v.push(s);
}

fn main() {
    let mut v = Vec::new();
    add(&mut v, "a");
    println!("{:?}", v);
}
