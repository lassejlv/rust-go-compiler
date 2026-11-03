use std::ops::{Add, AddAssign, Div, Index, IndexMut, Mul, MulAssign, Neg, Not, Rem, Sub, SubAssign, BitAnd, BitOr, BitXor, Shl, Shr};
use std::cmp::Ordering;

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Default)]
struct V3 { x: f64, y: f64, z: f64 }
impl V3 { const fn new(x: f64, y: f64, z: f64) -> Self { V3 { x, y, z } } fn dot(self, o: V3) -> f64 { self.x * o.x + self.y * o.y + self.z * o.z } fn len(self) -> f64 { self.dot(self).sqrt() } }
impl Add for V3 { type Output = V3; fn add(self, o: V3) -> V3 { V3::new(self.x + o.x, self.y + o.y, self.z + o.z) } }
impl Sub for V3 { type Output = V3; fn sub(self, o: V3) -> V3 { V3::new(self.x - o.x, self.y - o.y, self.z - o.z) } }
impl Mul<f64> for V3 { type Output = V3; fn mul(self, k: f64) -> V3 { V3::new(self.x * k, self.y * k, self.z * k) } }
impl Mul<V3> for f64 { type Output = V3; fn mul(self, v: V3) -> V3 { v * self } }
impl Mul for V3 { type Output = V3; fn mul(self, o: V3) -> V3 { V3::new(self.y * o.z - self.z * o.y, self.z * o.x - self.x * o.z, self.x * o.y - self.y * o.x) } }
impl Div<f64> for V3 { type Output = V3; fn div(self, k: f64) -> V3 { V3::new(self.x / k, self.y / k, self.z / k) } }
impl Neg for V3 { type Output = V3; fn neg(self) -> V3 { V3::new(-self.x, -self.y, -self.z) } }
impl AddAssign for V3 { fn add_assign(&mut self, o: V3) { self.x += o.x; self.y += o.y; self.z += o.z; } }
impl SubAssign for V3 { fn sub_assign(&mut self, o: V3) { *self = *self - o; } }
impl MulAssign<f64> for V3 { fn mul_assign(&mut self, k: f64) { *self = *self * k; } }
impl<'a> Add<&'a V3> for &'a V3 { type Output = V3; fn add(self, o: &V3) -> V3 { *self + *o } }
impl Index<usize> for V3 { type Output = f64; fn index(&self, i: usize) -> &f64 { match i { 0 => &self.x, 1 => &self.y, 2 => &self.z, _ => panic!("bad index {}", i) } } }
impl IndexMut<usize> for V3 { fn index_mut(&mut self, i: usize) -> &mut f64 { match i { 0 => &mut self.x, 1 => &mut self.y, _ => &mut self.z } } }
impl std::iter::Sum for V3 { fn sum<I: Iterator<Item = V3>>(it: I) -> V3 { it.fold(V3::default(), |a, b| a + b) } }
impl<'a> std::iter::Sum<&'a V3> for V3 { fn sum<I: Iterator<Item = &'a V3>>(it: I) -> V3 { it.copied().sum() } }

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
struct Bits(u16);
impl BitAnd for Bits { type Output = Bits; fn bitand(self, o: Bits) -> Bits { Bits(self.0 & o.0) } }
impl BitOr for Bits { type Output = Bits; fn bitor(self, o: Bits) -> Bits { Bits(self.0 | o.0) } }
impl BitXor for Bits { type Output = Bits; fn bitxor(self, o: Bits) -> Bits { Bits(self.0 ^ o.0) } }
impl Not for Bits { type Output = Bits; fn not(self) -> Bits { Bits(!self.0) } }
impl Shl<u32> for Bits { type Output = Bits; fn shl(self, n: u32) -> Bits { Bits(self.0.rotate_left(n)) } }
impl Shr<u32> for Bits { type Output = Bits; fn shr(self, n: u32) -> Bits { Bits(self.0.rotate_right(n)) } }
impl std::fmt::Binary for Bits { fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result { std::fmt::Binary::fmt(&self.0, f) } }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Mod7(u8);
impl Add for Mod7 { type Output = Mod7; fn add(self, o: Mod7) -> Mod7 { Mod7((self.0 + o.0) % 7) } }
impl Mul for Mod7 { type Output = Mod7; fn mul(self, o: Mod7) -> Mod7 { Mod7((self.0 * o.0) % 7) } }
impl Rem<u8> for Mod7 { type Output = u8; fn rem(self, m: u8) -> u8 { self.0 % m } }
impl Neg for Mod7 { type Output = Mod7; fn neg(self) -> Mod7 { Mod7((7 - self.0) % 7) } }
impl PartialOrd for Mod7 { fn partial_cmp(&self, o: &Mod7) -> Option<Ordering> { if self.0 == 0 || o.0 == 0 { None } else { self.0.partial_cmp(&o.0) } } }

#[derive(Debug, PartialEq, PartialOrd)]
enum Grade { F, D(u8), C { plus: bool }, B(f32), A }

struct Grid { w: usize, cells: Vec<char> }
impl Index<(usize, usize)> for Grid { type Output = char; fn index(&self, (r, c): (usize, usize)) -> &char { &self.cells[r * self.w + c] } }
impl IndexMut<(usize, usize)> for Grid { fn index_mut(&mut self, (r, c): (usize, usize)) -> &mut char { &mut self.cells[r * self.w + c] } }

fn main() {
    let a = V3::new(1.0, 2.0, 3.0);
    let b = V3::new(-2.0, 0.5, 4.0);
    println!("{:?}\n{:?}\n{:?}", a + b, a - b, a * b);
    println!("{:?} {:?} {:?}", a * 2.0, 0.5 * b, -a / 4.0);
    println!("dot={} len={:.5} &+={:?}", a.dot(b), (a * b).len(), &a + &b);
    let mut c = a;
    c += b; c -= V3::new(1.0, 1.0, 1.0); c *= 3.0;
    c[1] = 100.0; c[2] += 0.25;
    println!("{:?} c[0]={} c[2]={}", c, c[0], c[2]);
    let pts = vec![a, b, c];
    let s1: V3 = pts.iter().sum();
    let s2: V3 = pts.into_iter().map(|p| p * 0.1).sum();
    println!("{:?} {:?}", s1, s2);
    println!("cmp {:?} {:?} {}", a.partial_cmp(&b), V3::new(1.0, f64::NAN, 0.0).partial_cmp(&V3::new(1.0, 2.0, 0.0)), a < V3::new(1.0, 2.0, 3.5));
    let x = Bits(0b1010_0000_1111_0001);
    let y = Bits(0b0110_1100_0000_1111);
    println!("{:016b} {:016b} {:016b} {:016b}", x & y, x | y, x ^ y, !x);
    println!("{:016b} {:016b} {}", x << 4, x >> 1, x < y);
    let m = Mod7(5);
    println!("{:?} {:?} {:?} {} {:?} {:?}", m + Mod7(4), m * Mod7(6), -m, m % 3, m.partial_cmp(&Mod7(0)), Mod7(2) > Mod7(1));
    let mut grades = vec![Grade::B(3.5), Grade::A, Grade::D(2), Grade::C { plus: true }, Grade::F, Grade::C { plus: false }, Grade::D(1), Grade::B(f32::NAN), Grade::B(2.0)];
    let cmps: Vec<Option<Ordering>> = grades.windows(2).map(|w| w[0].partial_cmp(&w[1])).collect();
    println!("{:?}", cmps);
    grades.retain(|g| g == g);
    grades.sort_by(|a, b| a.partial_cmp(b).unwrap());
    println!("{:?}", grades);
    let mut g = Grid { w: 4, cells: vec!['.'; 12] };
    g[(1, 2)] = '#'; g[(2, 3)] = '@'; g[(0, 0)] = '*';
    for r in 0..3 { println!("{}", (0..4).map(|c| g[(r, c)]).collect::<String>()); }
    println!("{:?}", (1, "b", 3.0) < (1, "b", 3.5));
    println!("{:?}", [1, 2, 3][..].cmp(&[1, 2][..]));
    println!("{:?}", "Zeta".cmp("alpha"));
    println!("{:?}", Some(1).cmp(&None));
    println!("{:?}", (Ok::<i32, i32>(5)).partial_cmp(&Err(0)));
}
