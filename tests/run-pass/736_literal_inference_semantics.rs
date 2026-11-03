// Integer/float literal type inference flowing backward from later uses,
// which changes wrapping, shifting, division and printing semantics.
use std::fmt::Debug;

fn takes_u8(x: u8) -> u8 { x }
fn takes_f32(x: f32) -> f32 { x }
fn show<T: Debug>(label: &str, x: T) { println!("{:<22} {:?} ({})", label, x, std::any::type_name::<T>()); }

struct Wrap<T>(T);
impl<T: Copy + std::ops::Add<Output = T>> Wrap<T> { fn double(&self) -> T { self.0 + self.0 } }

fn main() {
    let a = 255;
    let _ = takes_u8(a);
    show("a (u8 via call)", a.wrapping_add(1));

    let b = 255;
    show("b (i32 default)", b + 1);

    let c = 0xFF;
    let arr: [i8; 1] = [c as i8];
    show("c as i8", arr[0]);

    let mut v = Vec::new();
    v.push(1);
    v.push(2);
    let total: u64 = v.iter().sum();
    show("vec elem u64", v[0] << 40);
    show("total", total);

    let d = 7;
    let e: i64 = d;
    show("d / -2 (i64)", d / -2);
    show("e % -3", e % -3);

    let big = 1 << 20;
    let _: u32 = big;
    show("big leading_zeros", big.leading_zeros());

    let f = 0.1;
    let _ = takes_f32(f);
    show("f (f32)", f + 0.2);
    let g = 0.1;
    show("g (f64)", g + 0.2);

    let h = 3;
    let w = Wrap(h);
    let _check: u16 = w.double();
    show("Wrap<u16>", w.0.max(9));

    let i = -1i32;
    let j = i as u32;
    show("-1 as u32", j);
    let k = 300;
    show("300 as u8", k as u8);

    let m = [1, 2, 3];
    let n: &[u8] = &m;
    show("m sum wrapping", m.iter().fold(250, |acc: u8, x| acc.wrapping_add(*x)));
    show("n.len", n.len());

    let p = 2;
    let _r: i16 = p;
    let q = p.pow(10);
    show("p.pow(10) i16", q.checked_mul(40));

    let s = 1_000;
    let t: Option<u16> = Some(s);
    show("Some(s) * 70 checked", t.and_then(|x| x.checked_mul(70)));

    let x = 100;
    let y = if x > 50 { x } else { 0u8 };
    show("if-branch unify u8", y.checked_add(200));

    let z = 5;
    let closure = |n: i8| n * 2;
    show("closure i8", closure(z).checked_mul(20));

    let tup = (1, 2.5);
    let (_, fl): (u64, f32) = tup;
    show("tuple f32", fl / 3.0);
    show("tuple u64", tup.0.checked_sub(2));

    let mut acc = 0;
    for byte in "AZ".bytes() { acc = acc * 256 + byte as u32; }
    show("acc u32", acc);

    let shift_amt = 3;
    let shifted = 1u64 << shift_amt;
    show("shift amount default i32", (shift_amt, shifted));

    let neg = -128;
    let as_i8: i8 = neg;
    show("i8 neg checked_neg", as_i8.checked_neg());
    show("i8 abs wrapping", as_i8.wrapping_abs());

    let lit_char = b'a';
    show("byte literal", lit_char + 1);
    let diff = b'z' - lit_char;
    show("byte diff", diff);
    let float_from_int = 7 as f32 / 2 as f32;
    show("int as f32 division", float_from_int);
    let float_inferred = 1e3;
    let _: f32 = float_inferred;
    show("1e3 f32 recip", 1.0 / float_inferred);
    let mx = std::cmp::max(3, 7u8);
    show("max(3, 7u8)", mx);
    let pos = [10, 20, 30].iter().position(|&x| x == 20).unwrap();
    show("position usize", pos);
    let sum_i = (1..=10).map(|x| x * x).sum::<i64>();
    show("sum::<i64>", sum_i);
    let maybe = "42".parse().unwrap_or(0) + 1u16;
    show("parse inferred u16", maybe);
}
