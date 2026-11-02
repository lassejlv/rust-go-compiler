mod primitive { pub use bool; pub use u64; pub use str; pub use f32; }
fn main() { let x: primitive::u64 = 5; let s: &primitive::str = "hi"; let b: primitive::bool = true; println!("{} {} {} {}", x, s, b, primitive::u64::MAX); println!("{}", <primitive::f32>::from(2u8)); }
