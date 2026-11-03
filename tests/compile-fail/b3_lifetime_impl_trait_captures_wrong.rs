//~ ERROR lifetime
fn longest<'a, 'b>(a: &'a str, b: &'b str) -> &'a str { if a.len() > b.len() { a } else { b } }
fn main() { longest("a", "bb"); }
