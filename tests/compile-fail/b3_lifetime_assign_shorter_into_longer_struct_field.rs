//~ ERROR lifetime
struct Ctx<'a> { name: &'a str }
fn set<'a, 'b>(c: &mut Ctx<'a>, n: &'b str) { c.name = n; }
fn main() { let mut c = Ctx { name: "a" }; set(&mut c, "b"); let _ = c.name; }
