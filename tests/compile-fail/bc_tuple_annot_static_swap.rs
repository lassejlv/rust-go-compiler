//~ ERROR lifetime
fn k<'s>(src: &'s str) -> &'static str {
    let t: (&'static str, &'s str) = ("x", src);
    let u: (&'s str, &'static str) = t;
    u.1
}
fn main() { println!("{:?}", k("a")); }
