use std::fmt;
use std::iter::{Product, Sum};
use std::ops::{Add, AddAssign, Index, IndexMut, Mul, Neg, Not, Sub, Range, Shl, BitXor, Rem};

#[derive(Debug, Clone, Copy, PartialEq, Default)]
struct V2 {
    x: f64,
    y: f64,
}

impl Add for V2 {
    type Output = V2;
    fn add(self, o: V2) -> V2 {
        V2 { x: self.x + o.x, y: self.y + o.y }
    }
}
impl<'a> Add<&'a V2> for &'a V2 {
    type Output = V2;
    fn add(self, o: &V2) -> V2 {
        *self + *o
    }
}
impl Sub for V2 {
    type Output = V2;
    fn sub(self, o: V2) -> V2 {
        V2 { x: self.x - o.x, y: self.y - o.y }
    }
}
impl Mul<f64> for V2 {
    type Output = V2;
    fn mul(self, k: f64) -> V2 {
        V2 { x: self.x * k, y: self.y * k }
    }
}
impl Mul<V2> for f64 {
    type Output = V2;
    fn mul(self, v: V2) -> V2 {
        v * self
    }
}
impl Mul for V2 {
    type Output = f64;
    fn mul(self, o: V2) -> f64 {
        self.x * o.x + self.y * o.y
    }
}
impl Neg for V2 {
    type Output = V2;
    fn neg(self) -> V2 {
        V2 { x: -self.x, y: -self.y }
    }
}
impl AddAssign for V2 {
    fn add_assign(&mut self, o: V2) {
        self.x += o.x;
        self.y += o.y;
    }
}
impl Sum for V2 {
    fn sum<I: Iterator<Item = V2>>(it: I) -> V2 {
        it.fold(V2::default(), |a, b| a + b)
    }
}
impl<'a> Sum<&'a V2> for V2 {
    fn sum<I: Iterator<Item = &'a V2>>(it: I) -> V2 {
        it.copied().sum()
    }
}
impl fmt::Display for V2 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let p = f.precision().unwrap_or(1);
        write!(f, "({:.*}, {:.*})", p, self.x, p, self.y)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
struct Mod7(u8);
impl Add for Mod7 {
    type Output = Self;
    fn add(self, o: Self) -> Self {
        Mod7((self.0 + o.0) % 7)
    }
}
impl Mul for Mod7 {
    type Output = Self;
    fn mul(self, o: Self) -> Self {
        Mod7((self.0 * o.0) % 7)
    }
}
impl Product for Mod7 {
    fn product<I: Iterator<Item = Self>>(it: I) -> Self {
        it.fold(Mod7(1), |a, b| a * b)
    }
}
impl Not for Mod7 {
    type Output = Mod7;
    fn not(self) -> Mod7 {
        Mod7((7 - self.0) % 7)
    }
}
impl Shl<u32> for Mod7 {
    type Output = Mod7;
    fn shl(self, n: u32) -> Mod7 {
        Mod7(((self.0 as u32) << n) as u8 % 7)
    }
}
impl BitXor for Mod7 {
    type Output = Mod7;
    fn bitxor(self, o: Mod7) -> Mod7 {
        Mod7((self.0 ^ o.0) % 7)
    }
}
impl Rem<u8> for Mod7 {
    type Output = u8;
    fn rem(self, m: u8) -> u8 {
        self.0 % m
    }
}

struct Grid {
    w: usize,
    cells: Vec<char>,
}
impl Index<(usize, usize)> for Grid {
    type Output = char;
    fn index(&self, (r, c): (usize, usize)) -> &char {
        &self.cells[r * self.w + c]
    }
}
impl IndexMut<(usize, usize)> for Grid {
    fn index_mut(&mut self, (r, c): (usize, usize)) -> &mut char {
        &mut self.cells[r * self.w + c]
    }
}
impl Index<usize> for Grid {
    type Output = [char];
    fn index(&self, r: usize) -> &[char] {
        &self.cells[r * self.w..(r + 1) * self.w]
    }
}
impl Index<Range<usize>> for Grid {
    type Output = [char];
    fn index(&self, r: Range<usize>) -> &[char] {
        &self.cells[r]
    }
}

fn generic_sum<T: Add<Output = T> + Copy + Default>(xs: &[T]) -> T {
    xs.iter().fold(T::default(), |a, &b| a + b)
}

fn dot_generic<T>(a: &[T], b: &[T]) -> T
where
    T: Mul<Output = T> + Sum<T> + Copy,
{
    a.iter().zip(b).map(|(&x, &y)| x * y).sum()
}

fn add_refs<'a, T>(a: &'a T, b: &'a T) -> <&'a T as Add<&'a T>>::Output
where
    &'a T: Add<&'a T>,
{
    a + b
}

fn main() {
    let a = V2 { x: 1.0, y: 2.0 };
    let b = V2 { x: 0.5, y: -1.0 };
    println!("{} {} {} {}", a + b, &a + &b, a - b, -a);
    println!("{} {} {:.3}", a * 2.0, 3.0 * b, a * b);
    let mut c = a;
    c += b;
    c += V2 { x: 10.0, ..Default::default() };
    println!("{:.2}", c);
    let vs = vec![a, b, c];
    let s1: V2 = vs.iter().sum();
    let s2: V2 = vs.into_iter().map(|v| v * 0.5).sum();
    println!("{} {}", s1, s2);
    println!("{}", add_refs(&a, &b));
    println!("{} {}", add_refs(&3, &4), generic_sum(&[1.5, 2.5]));
    println!("{}", dot_generic(&[1, 2, 3], &[4, 5, 6]));

    let ms: Vec<Mod7> = (1..6).map(Mod7).collect();
    let p: Mod7 = ms.iter().copied().product();
    println!("{:?} {:?} {:?}", p, !Mod7(3), Mod7(5) << 2);
    println!("{:?} {} {:?}", Mod7(6) ^ Mod7(3), Mod7(5) % 3, ms.iter().max());
    println!("{:?}", generic_sum(&[Mod7(3), Mod7(5), Mod7(6)]));

    let mut g = Grid { w: 3, cells: "abcdefghi".chars().collect() };
    g[(1, 1)] = 'X';
    println!("{} {:?} {:?}", g[(1, 1)], &g[2], &g[0..4]);
    let row: String = g[1].iter().collect();
    println!("{}", row);
    g[(0, 0)].make_ascii_uppercase();
    println!("{}", g.cells.iter().collect::<String>());

    // operators on primitives through references
    let x = 5;
    let y = &x;
    let z = &&7;
    println!("{} {} {} {}", y + 1, *y * **z, y - *z, &10 % y);
    let mut acc = 0u64;
    for v in &[1u64, 2, 3] {
        acc += v;
        acc *= *v;
    }
    println!("{}", acc);
    let bits = !0u8 ^ 0b1010;
    println!("{:08b} {} {}", bits, -(-5i32), !true);
    let strs = String::from("a") + "b" + &String::from("c");
    println!("{}", strs);
    use std::cmp::Ordering;
    println!("{:?} {:?}", 1.cmp(&2).reverse(), Ordering::Equal.then(Ordering::Less));
    println!("{:?}", (1, "b").cmp(&(1, "a")));
    println!("{:?}", [1, 2].partial_cmp(&[1, 3]));
    println!("{}", std::cmp::max_by_key(-3, 2, |x: &i32| x.abs()));
    println!("{} {}", std::cmp::min("x", "abc"), 5.clamp(1, 3));
}
