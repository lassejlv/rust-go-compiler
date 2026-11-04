use std::fmt::Debug;

trait Combine<Rhs = Self> { type Output; fn combine(self, r: Rhs) -> Self::Output; }
#[derive(Debug, Clone, Copy, PartialEq)]
struct P(i32, i32);
impl Combine for P { type Output = P; fn combine(self, r: P) -> P { P(self.0 + r.0, self.1 + r.1) } }
impl Combine<i32> for P { type Output = P; fn combine(self, r: i32) -> P { P(self.0 * r, self.1 * r) } }
impl Combine<P> for i32 { type Output = i64; fn combine(self, r: P) -> i64 { (self * r.0 * r.1) as i64 } }
impl<'a> Combine<&'a str> for String { type Output = String; fn combine(mut self, r: &'a str) -> String { self.push_str(r); self } }

fn combine_all<T: Combine<Output = T> + Copy>(init: T, xs: &[T]) -> T { xs.iter().fold(init, |a, b| a.combine(*b)) }
fn mixed<A, B>(a: A, b: B) -> <A as Combine<B>>::Output where A: Combine<B> { a.combine(b) }
fn scaled<T>(t: T, k: i32) -> T::Output where T: Combine<i32> { t.combine(k) }

trait Conv<To = String> { fn conv(&self) -> To; }
impl Conv for u8 { fn conv(&self) -> String { format!("u8:{}", self) } }
impl Conv<f64> for u8 { fn conv(&self) -> f64 { *self as f64 / 2.0 } }
impl<T: Conv> Conv for Vec<T> { fn conv(&self) -> String { self.iter().map(|x| x.conv()).collect::<Vec<_>>().join("+") } }

struct Matrix<T = f64, const R: usize = 2, const C: usize = 2> { d: [[T; C]; R] }
impl<T: Copy + Debug + Default + std::ops::Add<Output = T>, const R: usize, const C: usize> Matrix<T, R, C> {
    fn add(&self, o: &Self) -> Self {
        let mut d = [[T::default(); C]; R];
        for r in 0..R { for c in 0..C { d[r][c] = self.d[r][c] + o.d[r][c]; } }
        Matrix { d }
    }
}


fn main() {
    println!("{:?}", P(1, 2).combine(P(3, 4)));
    println!("{:?}", P(1, 2).combine(10));
    println!("{}", 2.combine(P(3, 4)));
    println!("{}", String::from("ab").combine("cd"));
    println!("{:?}", combine_all(P(0, 0), &[P(1, 1), P(2, 3)]));
    println!("{:?} {} {:?}", mixed(P(1, 1), 3), mixed(3, P(2, 2)), scaled(P(5, -5), 2));
    let s: String = 4u8.conv();
    let f: f64 = 4u8.conv();
    println!("{} {} {} {}", s, f, vec![1u8, 2].conv(), <u8 as Conv<f64>>::conv(&9));
    let m: Matrix = Matrix { d: [[1.0, 2.0], [3.0, 4.0]] };
    let m2 = m.add(&m);
    println!("{:?}", m2.d);
    let mi: Matrix<i32, 1, 3> = Matrix { d: [[1, 2, 3]] };
    println!("{:?}", mi.add(&mi).d);
}
