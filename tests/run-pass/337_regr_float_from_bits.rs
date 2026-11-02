pub trait Float: Sized + Copy {
    const MANTISSA_SIZE: i32;
    fn from_bits(u: u64) -> Self;
    fn to_bits(self) -> u64;
}
impl Float for f32 {
    const MANTISSA_SIZE: i32 = 23;
    fn from_bits(u: u64) -> f32 { f32::from_bits(u as u32) }
    fn to_bits(self) -> u64 { f32::to_bits(self) as u64 }
}
impl Float for f64 {
    const MANTISSA_SIZE: i32 = 52;
    fn from_bits(u: u64) -> f64 { f64::from_bits(u) }
    fn to_bits(self) -> u64 { f64::to_bits(self) }
}
fn conv<F: Float>(mant: u64, exp: i32) -> F {
    let mut word = mant;
    word |= (exp as u64) << F::MANTISSA_SIZE;
    F::from_bits(word)
}
fn main() {
    let a: f32 = conv(0, 127); let b: f64 = conv(1 << 51, 1023);
    println!("{} {} {}", a, b, Float::to_bits(a));
}
