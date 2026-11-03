use std::fmt;
struct P;
impl fmt::Display for P { fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result { write!(f, "P") } }
#[derive(Debug, Default, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
enum E { #[default] A, B(i32), C { x: u8 } }
struct S<const N: usize = 3>;
impl<const N: usize> S<N> { fn n(&self) -> usize { N } }
trait T { fn make() -> impl Iterator<Item = u32>; fn it(&self) -> impl fmt::Debug { 5 } }
impl T for P { fn make() -> impl Iterator<Item = u32> { 0..3 } }
fn main() {
    println!("[{:>5}] [{:<5}] [{:^5}]", P, P, P);
    println!("{:?} {:?}", E::default(), [E::C{x:1}, E::B(2), E::A].iter().max());
    let s: S = S; println!("{} {}", s.n(), S::<7>.n());
    println!("{:?} {:?}", P::make().collect::<Vec<_>>(), P.it());
    let x = const { 5 * 3 };
    println!("{}", x);
    let arr: [String; 3] = std::array::from_fn(|i| format!("s{}", i));
    println!("{:?} {:?}", arr, arr.each_ref().map(|s| s.len()));
    let v = [1, 2, 3, 4, 5, 6, 7];
    let (ch, rem) = v.as_chunks::<3>();
    println!("{:?} {:?}", ch, rem);
    println!("{:?}", std::iter::repeat_n("ab", 3).collect::<String>());
    println!("{:?}", v.iter().rev().skip_while(|&&x| x > 4).map(|x| x * 2).collect::<Vec<_>>());
    let mut it = (1..=3u8).into_iter();
    println!("{:?} {:?}", it.next_back(), it.len());
    println!("{}", i32::MAX.wrapping_add(1).checked_ilog2().is_none());
    let mut w = String::new();
    use std::fmt::Write;
    write!(w, "{:>+08.2}|{:<#6x}|{:^+7}", -3.14159, 26, 5).unwrap();
    println!("{}", w);
    println!("{:?}", format_args!("{} {}", 1, 2));
    println!("{}", format!("{:?}", "a\u{200b}b\u{301}"));
    println!("{:5}|{:<5}|{:>5?}", 'x', '\u{301}', 'y');
    println!("{:e} {:E} {:e}", 12345678901234567890u64, 0.000123f32, 1u8);
    println!("{:.3e} {:10.2e}|", 123456u32, 9999i32);
    println!("{:?} {:?}", f32::MIN_POSITIVE, 1.0e-45f32);
    println!("{} {}", 1.0e-45f32, f64::MIN_POSITIVE / 2.0);
    println!("{:?}", 0.1f32 as f64);
    println!("{:.20}", 0.1f64);
    println!("{:.30}", 1e-20f64);
    println!("{}", 123456789012345680000000000000.0f64);
    println!("{:?} {}", 1e300 * 10.0f64, -1.0f64 / 0.0);
    println!("{:08.3} {:+08.3} {:08.3}", -0.5f64, 0.0f64, f64::NAN);
    println!("{:08} {:+08}", f64::INFINITY, -f64::INFINITY);
    println!("{:#010b} {:#06o} {:#x}", -5i8, 8u16, i64::MIN);
    println!("{:x} {:o} {:b}", -1i128, u128::MAX, i16::MIN);
}
