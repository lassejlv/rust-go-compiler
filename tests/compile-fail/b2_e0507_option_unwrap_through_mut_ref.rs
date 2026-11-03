//~ ERROR E0507
struct Slot { v: Option<String> }
fn get(s: &mut Slot) -> String {
    s.v.unwrap()
}
fn main() { let mut s = Slot { v: Some("x".into()) }; println!("{}", get(&mut s)); }
