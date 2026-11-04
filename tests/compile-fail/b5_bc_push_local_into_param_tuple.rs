//~ ERROR E0597
// a local's borrow stored into the referent of a `&mut` parameter
fn f<'a>(v: &mut Vec<(&'static str, &'a str)>) {
    let s = String::new();
    v.push(("k", &s));
}
fn main() {
    let mut v = Vec::new();
    f(&mut v);
    println!("{:?}", v);
}
