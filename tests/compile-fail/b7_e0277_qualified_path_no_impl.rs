//~ ERROR E0277
// `<u8 as Trait>::Out` where `u8` does not implement `Trait`.
trait Trait {
    type Out;
}
impl Trait for i32 {
    type Out = u64;
}
fn main() {
    let x: <u8 as Trait>::Out = 1;
    println!("{}", x);
}
