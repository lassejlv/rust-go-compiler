use std::fmt::{self, Debug, Display};
use std::ops::{Add, Mul};

trait Monoid: Sized {
    const IDENTITY: Self;
    const NAME: &'static str;
    fn combine(self, other: Self) -> Self;
    fn concat<I: IntoIterator<Item = Self>>(it: I) -> Self { it.into_iter().fold(Self::IDENTITY, Self::combine) }
}

#[derive(Debug, Clone, Copy, PartialEq)] struct Sum(i64);
#[derive(Debug, Clone, Copy, PartialEq)] struct Prod(i64);
#[derive(Debug, Clone, Copy, PartialEq)] struct MaxM(i64);

impl Monoid for Sum { const IDENTITY: Self = Sum(0); const NAME: &'static str = "sum"; fn combine(self, o: Self) -> Self { Sum(self.0 + o.0) } }
impl Monoid for Prod { const IDENTITY: Self = Prod(1); const NAME: &'static str = "prod"; fn combine(self, o: Self) -> Self { Prod(self.0 * o.0) } }
impl Monoid for MaxM { const IDENTITY: Self = MaxM(i64::MIN); const NAME: &'static str = "max"; fn combine(self, o: Self) -> Self { if o.0 > self.0 { o } else { self } } }
impl<A: Monoid, B: Monoid> Monoid for (A, B) {
    const IDENTITY: Self = (A::IDENTITY, B::IDENTITY);
    const NAME: &'static str = "pair";
    fn combine(self, o: Self) -> Self { (self.0.combine(o.0), self.1.combine(o.1)) }
}

trait Container {
    type Item;
    type Iter<'a>: Iterator<Item = &'a Self::Item> where Self: 'a;
    const KIND: &'static str;
    fn items<'a>(&'a self) -> Self::Iter<'a>;
    fn first_item(&self) -> Option<&Self::Item> { self.items().next() }
}

struct Stack<T> { v: Vec<T> }
struct Pair<T> { a: T, b: T }

impl<T> Container for Stack<T> {
    type Item = T;
    type Iter<'a> = std::iter::Rev<std::slice::Iter<'a, T>> where T: 'a;
    const KIND: &'static str = "stack";
    fn items<'a>(&'a self) -> Self::Iter<'a> { self.v.iter().rev() }
}
impl<T> Container for Pair<T> {
    type Item = T;
    type Iter<'a> = std::array::IntoIter<&'a T, 2> where T: 'a;
    const KIND: &'static str = "pair";
    fn items<'a>(&'a self) -> Self::Iter<'a> { [&self.a, &self.b].into_iter() }
}

fn show<C: Container>(c: &C) -> String where C::Item: Debug {
    let parts: Vec<String> = c.items().map(|x| format!("{:?}", x)).collect();
    format!("{}[{}] first={:?}", C::KIND, parts.join(","), c.first_item())
}

trait Shape { const SIDES: u32; fn name() -> String { format!("{}-gon", Self::SIDES) } }
struct Tri; struct Sq; struct Hex;
impl Shape for Tri { const SIDES: u32 = 3; fn name() -> String { "triangle".into() } }
impl Shape for Sq { const SIDES: u32 = 4; }
impl Shape for Hex { const SIDES: u32 = 6; }
fn total_sides<A: Shape, B: Shape>() -> u32 { A::SIDES + B::SIDES }

struct Matrix<T, const R: usize, const C: usize> { d: [[T; C]; R] }
impl<T: Copy + Default + Add<Output = T> + Mul<Output = T>, const R: usize, const C: usize> Matrix<T, R, C> {
    const CELLS: usize = R * C;
    fn mul<const K: usize>(&self, o: &Matrix<T, C, K>) -> Matrix<T, R, K> {
        let mut d = [[T::default(); K]; R];
        for i in 0..R { for j in 0..K { for k in 0..C { d[i][j] = d[i][j] + self.d[i][k] * o.d[k][j]; } } }
        Matrix { d }
    }
    fn transpose(&self) -> Matrix<T, C, R> {
        let mut d = [[T::default(); R]; C];
        for i in 0..R { for j in 0..C { d[j][i] = self.d[i][j]; } }
        Matrix { d }
    }
}
impl<T: Display, const R: usize, const C: usize> Display for Matrix<T, R, C> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        for row in &self.d {
            let s: Vec<String> = row.iter().map(|x| format!("{:>4}", x)).collect();
            writeln!(f, "|{}|", s.join(""))?;
        }
        Ok(())
    }
}

trait Convert<T> { fn convert(&self) -> T; }
impl Convert<String> for i32 { fn convert(&self) -> String { format!("i32:{}", self) } }
impl Convert<f64> for i32 { fn convert(&self) -> f64 { *self as f64 / 2.0 } }
impl Convert<Vec<u8>> for i32 { fn convert(&self) -> Vec<u8> { self.to_le_bytes().to_vec() } }

fn main() {
    println!("{:?} {:?} {:?}", Sum::concat((1..=10).map(Sum)), Prod::concat((1..=10).map(Prod)), MaxM::concat([3, -2, 9, 4].map(MaxM)));
    let pairs = vec![(Sum(1), Prod(2)), (Sum(3), Prod(4)), (Sum(5), Prod(6))];
    println!("{:?} names {} {} {}", <(Sum, Prod)>::concat(pairs), Sum::NAME, <(Sum, MaxM)>::NAME, MaxM::NAME);
    println!("{:?}", <(Sum, (Prod, MaxM))>::IDENTITY);
    let st = Stack { v: vec!["a", "b", "c"] };
    let pr = Pair { a: 1.5, b: -2.5 };
    println!("{}\n{}", show(&st), show(&pr));
    let empty: Stack<u8> = Stack { v: vec![] };
    println!("{}", show(&empty));
    println!("{} {} {} total={} {}", Tri::name(), Sq::name(), Hex::name(), total_sides::<Tri, Hex>(), total_sides::<Sq, Sq>());
    let a = Matrix { d: [[1, 2, 3], [4, 5, 6]] };
    let b = Matrix { d: [[7, 8], [9, 10], [11, 12]] };
    let c = a.mul(&b);
    print!("{}", c);
    print!("{}", a.transpose());
    println!("cells {} {} {}", Matrix::<i32, 2, 3>::CELLS, Matrix::<f64, 4, 4>::CELLS, Matrix::<u8, 1, 7>::CELLS);
    let f = Matrix { d: [[0.5, 1.0], [1.5, 2.0]] };
    print!("{}", f.mul(&f.transpose()));
    let s: String = 42.convert();
    let x: f64 = 42.convert();
    let v: Vec<u8> = (-2).convert();
    println!("{} {} {:?} {}", s, x, v, <i32 as Convert<String>>::convert(&7));
}
