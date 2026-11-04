//~ ERROR E0597
fn f<'a>(v: &mut Vec<&'a str>) {
    let s = String::from("x");
    v.push(&s);
}
fn main() {
    let mut v = Vec::new();
    f(&mut v);
    println!("{:?}", v);
}
