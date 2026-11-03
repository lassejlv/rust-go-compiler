//~ ERROR E0133
union U { i: u32, f: f32 }
fn main() { let u = U { i: 1 }; println!("{}", u.f); }
