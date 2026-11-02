// Checked/saturating/wrapping/overflowing arithmetic paths across integer widths, generic over
// a trait implemented by macro, plus a safe "Money" type that refuses to overflow.
use std::fmt::Debug;

trait CheckedOps: Copy + Debug {
    const MIN: Self;
    const MAX: Self;
    const NAME: &'static str;
    fn c_add(self, o: Self) -> Option<Self>;
    fn c_sub(self, o: Self) -> Option<Self>;
    fn c_mul(self, o: Self) -> Option<Self>;
    fn c_div(self, o: Self) -> Option<Self>;
    fn s_add(self, o: Self) -> Self;
    fn s_mul(self, o: Self) -> Self;
    fn w_add(self, o: Self) -> Self;
    fn w_mul(self, o: Self) -> Self;
    fn o_add(self, o: Self) -> (Self, bool);
    fn o_sub(self, o: Self) -> (Self, bool);
    fn from_i8(v: i8) -> Self;
}

macro_rules! impl_checked {
    ($($t:ty),*) => {$(
        impl CheckedOps for $t {
            const MIN: Self = <$t>::MIN;
            const MAX: Self = <$t>::MAX;
            const NAME: &'static str = stringify!($t);
            fn c_add(self, o: Self) -> Option<Self> { self.checked_add(o) }
            fn c_sub(self, o: Self) -> Option<Self> { self.checked_sub(o) }
            fn c_mul(self, o: Self) -> Option<Self> { self.checked_mul(o) }
            fn c_div(self, o: Self) -> Option<Self> { self.checked_div(o) }
            fn s_add(self, o: Self) -> Self { self.saturating_add(o) }
            fn s_mul(self, o: Self) -> Self { self.saturating_mul(o) }
            fn w_add(self, o: Self) -> Self { self.wrapping_add(o) }
            fn w_mul(self, o: Self) -> Self { self.wrapping_mul(o) }
            fn o_add(self, o: Self) -> (Self, bool) { self.overflowing_add(o) }
            fn o_sub(self, o: Self) -> (Self, bool) { self.overflowing_sub(o) }
            fn from_i8(v: i8) -> Self { v as $t }
        }
    )*};
}
impl_checked!(u8, i8, u16, i16, u32, i32, u64, i64, usize, isize);

fn probe<T: CheckedOps>() {
    let one = T::from_i8(1);
    let two = T::from_i8(2);
    let m1 = T::from_i8(-1);
    println!(
        "{:<5} max+1={:?} min-1={:?} max*2={:?} min/-1={:?} sat={:?},{:?} wrap={:?},{:?} ovf={:?},{:?}",
        T::NAME,
        T::MAX.c_add(one),
        T::MIN.c_sub(one),
        T::MAX.c_mul(two),
        T::MIN.c_div(m1),
        T::MAX.s_add(one),
        T::MAX.s_mul(two),
        T::MAX.w_add(one),
        T::MAX.w_mul(two),
        T::MAX.o_add(one),
        T::MIN.o_sub(one),
    );
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct Cents(i64);

#[derive(Debug, PartialEq)]
enum MoneyError {
    Overflow(&'static str),
    DivideByZero,
    Negative,
}

impl Cents {
    fn add(self, o: Cents) -> Result<Cents, MoneyError> {
        self.0.checked_add(o.0).map(Cents).ok_or(MoneyError::Overflow("add"))
    }
    fn sub_nonneg(self, o: Cents) -> Result<Cents, MoneyError> {
        let r = self.0.checked_sub(o.0).ok_or(MoneyError::Overflow("sub"))?;
        if r < 0 {
            Err(MoneyError::Negative)
        } else {
            Ok(Cents(r))
        }
    }
    fn scale(self, num: i64, den: i64) -> Result<Cents, MoneyError> {
        if den == 0 {
            return Err(MoneyError::DivideByZero);
        }
        let wide = (self.0 as i128) * (num as i128) / (den as i128);
        i64::try_from(wide).map(Cents).map_err(|_| MoneyError::Overflow("scale"))
    }
    fn split(self, n: i64) -> Result<Vec<Cents>, MoneyError> {
        let base = self.0.checked_div(n).ok_or(MoneyError::DivideByZero)?;
        let rem = self.0.checked_rem(n).ok_or(MoneyError::DivideByZero)?;
        Ok((0..n).map(|i| Cents(base + if i < rem { 1 } else { 0 })).collect())
    }
}

fn sum_checked(xs: &[u8]) -> Option<u8> {
    xs.iter().try_fold(0u8, |acc, &x| acc.checked_add(x))
}

fn main() {
    probe::<u8>();
    probe::<i8>();
    probe::<u16>();
    probe::<i16>();
    probe::<u32>();
    probe::<i32>();
    probe::<u64>();
    probe::<i64>();
    probe::<usize>();
    probe::<isize>();

    println!("{:?} {:?}", 200u8.checked_add(55), 200u8.checked_add(56));
    println!("{:?} {:?} {:?}", 5u32.checked_sub(6), (-128i8).checked_abs(), (-128i8).checked_neg());
    println!("{:?} {:?}", 10i32.checked_rem(0), 10i32.checked_rem(3));
    println!("{:?} {:?}", 2u32.checked_pow(31), 2u32.checked_pow(32));
    println!("{} {} {}", 2i32.saturating_pow(40), (-2i32).saturating_pow(41), 100u8.saturating_sub(200));
    println!("{} {} {}", (-128i8).wrapping_abs(), (-128i8).wrapping_div(-1), 3u8.wrapping_sub(5));
    println!("{:?} {:?}", 255u8.overflowing_mul(2), i32::MIN.overflowing_neg());
    println!("{} {} {}", (-7i32).div_euclid(2), (-7i32).rem_euclid(2), 7i32.div_euclid(-2));
    println!("{} {}", i64::MAX.abs_diff(i64::MIN), 3u8.abs_diff(250));
    println!("{:?} {:?}", u8::try_from(300i32).is_err(), i8::try_from(-128i64));
    println!("{:?} {:?}", u16::try_from(-1i8).map_err(|e| e.to_string()), usize::try_from(42u64));
    println!("{} {} {}", 300i32 as u8, -1i32 as u32, 3.99f64 as i8);
    println!("{} {} {}", f64::NAN as i32, 1e20 as i32, -1e20 as i64);
    println!("{:?} {:?}", sum_checked(&[100, 100, 55]), sum_checked(&[100, 100, 56]));

    let price = Cents(1999);
    println!("{:?}", price.add(Cents(1)));
    println!("{:?}", Cents(i64::MAX).add(Cents(1)));
    println!("{:?} {:?}", price.sub_nonneg(Cents(2000)), price.sub_nonneg(Cents(999)));
    println!("{:?} {:?}", price.scale(3, 2), Cents(i64::MAX).scale(2, 1));
    println!("{:?} {:?}", price.scale(1, 0), Cents(100).split(3));
    println!("{:?}", Cents(10).split(0));
    println!("{:?}", Cents(i64::MIN).0.checked_div(-1));
}
