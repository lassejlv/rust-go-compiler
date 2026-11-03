use std::ops::{Add, Mul};

fn add_all<T: Add<Output = T> + Copy>(xs: &[T], z: T) -> T { xs.iter().fold(z, |a, &b| a + b) }
fn mul_all<T: Mul<Output = T> + Copy>(xs: &[T], one: T) -> T { xs.iter().fold(one, |a, &b| a * b) }

trait Checked: Sized + Copy { fn cadd(self, o: Self) -> Option<Self>; fn wadd(self, o: Self) -> Self; fn sadd(self, o: Self) -> Self; }
macro_rules! ck { ($($t:ty)*) => { $( impl Checked for $t {
    fn cadd(self, o: Self) -> Option<Self> { self.checked_add(o) }
    fn wadd(self, o: Self) -> Self { self.wrapping_add(o) }
    fn sadd(self, o: Self) -> Self { self.saturating_add(o) }
} )* } }
ck!(u8 i8 u16 i32 u64 i128);

fn report<T: Checked + std::fmt::Debug>(a: T, b: T) {
    println!("{:?} {:?} {:?}", a.cadd(b), a.wadd(b), a.sadd(b));
}

fn main() {
    println!("{}", add_all(&[100u8, 100, 50], 0));
    println!("{}", add_all(&[1i64 << 40, 1 << 40], 0));
    println!("{}", mul_all(&[3u128, 1 << 100], 1));
    report(250u8, 10);
    report(-100i8, -100);
    report(i32::MAX, 1);
    report(u64::MAX - 1, 1);
    report(i128::MIN, -1);
    report(60000u16, 6000);
    println!("{}", add_all(&[200u8, 100], 0));
}
