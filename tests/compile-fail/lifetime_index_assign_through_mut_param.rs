//~ ERROR lifetime may not live long enough
fn put<'a, 'b>(v: &mut Vec<&'a str>, s: &'b str) {
    v[0] = s;
}

fn main() {
    let mut v = vec!["a"];
    put(&mut v, "b");
    println!("{:?}", v);
}
