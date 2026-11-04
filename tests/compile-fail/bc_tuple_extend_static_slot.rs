//~ ERROR lifetime
fn g<'s>(src: &'s str) -> Vec<(&'static str, &'s str)> {
    let mut out = Vec::new();
    out.extend(vec![(src, "x")]);
    out
}
fn main() { println!("{:?}", g("a")); }
