//~ ERROR lifetime
struct Two<'a, 'b> { a: &'a str, b: &'b str }
fn swap<'a, 'b>(t: Two<'a, 'b>) -> Two<'b, 'a> {
    Two { a: t.a, b: t.b }
}
fn main() { let t = swap(Two { a: "x", b: "y" }); println!("{}{}", t.a, t.b); }
