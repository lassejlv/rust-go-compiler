//~ ERROR lifetime
struct Parser<'a> { src: &'a str }
impl<'a> Parser<'a> {
    fn rest<'b>(&'b self, other: &'b str) -> &'a str { other }
}
fn main() { let p = Parser { src: "x" }; p.rest("y"); let _ = p.src; }
