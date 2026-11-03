//~ ERROR lifetime
struct Words<'a> { s: &'a str }
impl<'a> Words<'a> {
    fn first<'b>(&'b self) -> &'a str where 'a: 'b {
        let t: &'b str = self.s;
        t
    }
}
fn main() { let w = Words { s: "a b" }; println!("{}", w.first()); }
