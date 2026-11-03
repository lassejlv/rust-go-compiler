//~ ERROR E0515
fn longest<'a>(a: &'a str, b: &'a str) -> &'a str { if a.len() > b.len() { a } else { b } }
fn pick(x: &str) -> &str {
    let local = String::from("local value");
    longest(x, &local)
}
fn main() { println!("{}", pick("a")); }
