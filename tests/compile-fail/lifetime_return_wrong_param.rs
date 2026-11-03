//~ ERROR lifetime may not live long enough
fn pick<'a, 'b>(x: &'a str, _y: &'b str) -> &'a str {
    _y
}

fn main() {
    println!("{}", pick("a", "b"));
}
