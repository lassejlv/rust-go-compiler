//~ ERROR E0609
struct U { name: String }
fn main() { let u: Option<U> = None; println!("{}", u.name); }
