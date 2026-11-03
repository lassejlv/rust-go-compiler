//~ ERROR lifetime
struct Ctx<'a> { data: &'a str }
impl<'a> Ctx<'a> {
    fn pick<'b>(&self, other: &'b str) -> &'a str {
        if other.len() > 3 { other } else { self.data }
    }
}
fn main() { let c = Ctx { data: "d" }; println!("{}", c.pick("other")); }
