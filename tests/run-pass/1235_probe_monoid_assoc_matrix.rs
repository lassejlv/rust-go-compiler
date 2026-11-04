mod g01 {
    use std::collections::HashMap;
    use std::hash::Hash;
    trait Monoid { fn empty() -> Self; fn combine(&self, o: &Self) -> Self; }
    impl Monoid for i64 { fn empty() -> Self { 0 } fn combine(&self, o: &Self) -> Self { self + o } }
    impl Monoid for String { fn empty() -> Self { String::new() } fn combine(&self, o: &Self) -> Self { format!("{}{}", self, o) } }
    impl<T: Clone> Monoid for Vec<T> { fn empty() -> Self { vec![] } fn combine(&self, o: &Self) -> Self { let mut v = self.clone(); v.extend(o.iter().cloned()); v } }
    impl<A: Monoid, B: Monoid> Monoid for (A, B) { fn empty() -> Self { (A::empty(), B::empty()) } fn combine(&self, o: &Self) -> Self { (self.0.combine(&o.0), self.1.combine(&o.1)) } }
    impl<T: Monoid> Monoid for Option<T> { fn empty() -> Self { None } fn combine(&self, o: &Self) -> Self { match (self, o) { (Some(a), Some(b)) => Some(a.combine(b)), (Some(a), None) | (None, Some(a)) => Some(a.combine(&T::empty())), (None, None) => None } } }
    fn fold_all<M: Monoid, I: IntoIterator<Item = M>>(it: I) -> M { it.into_iter().fold(M::empty(), |a, b| a.combine(&b)) }
    fn group_by<K, V, I, F>(it: I, key: F) -> HashMap<K, V> where I: IntoIterator, F: Fn(&I::Item) -> K, K: Eq + Hash, V: Monoid + From<I::Item> {
        let mut m: HashMap<K, V> = HashMap::new();
        for x in it { let k = key(&x); let v = V::from(x); let e = m.entry(k).or_insert_with(V::empty); *e = e.combine(&v); }
        m
    }
    #[derive(Debug, Clone, Copy)] struct Sale { region: u8, amount: i64 }
    impl From<Sale> for i64 { fn from(s: Sale) -> i64 { s.amount } }
    pub fn main() {
        println!("{}", fold_all(vec![1i64, 2, 3]));
        println!("{:?}", fold_all(vec![(1i64, "a".to_string()), (2, "b".to_string())]));
        println!("{:?}", fold_all(vec![Some(vec![1]), None, Some(vec![2, 3])]));
        println!("{:?}", fold_all(Vec::<Option<i64>>::new()));
        let sales = vec![Sale { region: 1, amount: 10 }, Sale { region: 2, amount: 5 }, Sale { region: 1, amount: 7 }];
        let by: HashMap<u8, i64> = group_by(sales.iter().copied(), |s| s.region);
        let mut v: Vec<_> = by.into_iter().collect(); v.sort();
        println!("{:?}", v);
        let words = vec!["apple", "avocado", "banana"];
        let byc: HashMap<char, String> = group_by(words.into_iter().map(String::from), |w: &String| w.chars().next().unwrap());
        let mut v: Vec<_> = byc.into_iter().collect(); v.sort();
        println!("{:?}", v);
    }
}

mod g02 {
    use std::ops::Mul;
    trait Shape { type Unit: Copy + Mul<Output = Self::Unit> + Into<f64>; fn dims(&self) -> (Self::Unit, Self::Unit); fn area(&self) -> f64 { let (a, b) = self.dims(); (a * b).into() } }
    struct RectI(u16, u16);
    struct RectF(f32, f32);
    impl Shape for RectI { type Unit = u16; fn dims(&self) -> (u16, u16) { (self.0, self.1) } }
    impl Shape for RectF { type Unit = f32; fn dims(&self) -> (f32, f32) { (self.0, self.1) } }
    fn total_area<S: Shape>(shapes: &[S]) -> f64 { shapes.iter().map(Shape::area).sum() }
    fn biggest<'a, S: Shape>(shapes: &'a [S]) -> Option<&'a S> { shapes.iter().max_by(|a, b| a.area().partial_cmp(&b.area()).unwrap()) }
    fn unit_sum<S>(s: &[S]) -> S::Unit where S: Shape, S::Unit: std::iter::Sum<S::Unit> { s.iter().map(|x| x.dims().0).sum() }
    trait Container { type Item; fn get(&self, i: usize) -> Option<&Self::Item>; fn first(&self) -> Option<&Self::Item> { self.get(0) } }
    impl<T> Container for Vec<T> { type Item = T; fn get(&self, i: usize) -> Option<&T> { <[T]>::get(self, i) } }
    fn firsts<C: Container>(cs: &[C]) -> Vec<&C::Item> { cs.iter().filter_map(|c| c.first()).collect() }
    pub fn main() {
        println!("{} {}", total_area(&[RectI(2, 3), RectI(4, 5)]), total_area(&[RectF(0.5, 0.5)]));
        println!("{:?}", biggest(&[RectI(1, 1), RectI(3, 1), RectI(2, 1)]).map(|r| r.0));
        println!("{} {}", unit_sum(&[RectI(2, 3), RectI(4, 5)]), unit_sum(&[RectF(1.5, 0.0), RectF(2.0, 1.0)]));
        println!("{:?}", firsts(&[vec![1, 2], vec![], vec![3]]));
    }
}

mod g03 {
    use std::fmt::Debug;
    #[derive(Debug, Clone, PartialEq)]
    struct Matrix<T> { r: usize, c: usize, d: Vec<T> }
    impl<T: Copy + Default + std::ops::Add<Output = T> + std::ops::Mul<Output = T>> Matrix<T> {
        fn new(r: usize, c: usize) -> Self { Matrix { r, c, d: vec![T::default(); r * c] } }
        fn from(r: usize, c: usize, d: &[T]) -> Self { assert_eq!(d.len(), r * c); Matrix { r, c, d: d.to_vec() } }
        fn at(&self, i: usize, j: usize) -> T { self.d[i * self.c + j] }
        fn mul(&self, o: &Matrix<T>) -> Option<Matrix<T>> {
            if self.c != o.r { return None; }
            let mut m = Matrix::new(self.r, o.c);
            for i in 0..self.r { for j in 0..o.c { let mut s = T::default(); for k in 0..self.c { s = s + self.at(i, k) * o.at(k, j); } m.d[i * o.c + j] = s; } }
            Some(m)
        }
        fn pow(&self, n: u32) -> Matrix<T> where T: From<u8> {
            let mut res = Matrix::new(self.r, self.c);
            for i in 0..self.r { res.d[i * self.c + i] = T::from(1u8); }
            let mut base = self.clone();
            let mut n = n;
            while n > 0 { if n & 1 == 1 { res = res.mul(&base).unwrap(); } base = base.mul(&base).unwrap(); n >>= 1; }
            res
        }
    }
    impl<T: Debug> std::fmt::Display for Matrix<T> {
        fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
            for row in self.d.chunks(self.c) { writeln!(f, "{:?}", row)?; }
            Ok(())
        }
    }
    pub fn main() {
        let fib = Matrix::<u64>::from(2, 2, &[1, 1, 1, 0]);
        print!("{}", fib.pow(50));
        let a = Matrix::<f64>::from(2, 3, &[1.0, 2.0, 3.0, 4.0, 5.0, 6.0]);
        let b = Matrix::from(3, 1, &[1.0, 0.5, 0.25]);
        print!("{}", a.mul(&b).unwrap());
        println!("{:?}", a.mul(&a));
        let i = Matrix::<i32>::from(2, 2, &[2, -1, 0, 3]).pow(5);
        print!("{}", i);
    }
}

fn main() {
    println!("== g01");
    g01::main();
    println!("== g02");
    g02::main();
    println!("== g03");
    g03::main();
}
