//~ ERROR lifetime
struct Token<'a> {
    w: &'a str,
}
fn tokens<'a, 'b>(out: &mut Vec<Token<'a>>, b: &'b str) {
    out.extend(b.split(' ').map(|w| Token { w }));
}
fn main() {
    let mut out = Vec::new();
    tokens(&mut out, "a b");
    println!("{}", out.len());
}
