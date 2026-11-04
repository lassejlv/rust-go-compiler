// Assorted probes: generic repository/service with dyn, typestate builder,
// const float casts, async closures, operator impls on references, syntax,
// runtime overflow panics.
mod l01 {
    use std::collections::HashMap;
    use std::fmt::Debug;
    use std::hash::Hash;
    pub trait Repository<K: 'static, V: 'static> {
        fn get(&self, k: &K) -> Option<&V>;
        fn put(&mut self, k: K, v: V) -> Option<V>;
        fn all(&self) -> Box<dyn Iterator<Item = (&K, &V)> + '_>;
        fn find<P>(&self, p: P) -> Vec<&V> where P: Fn(&V) -> bool, Self: Sized { self.all().filter(|(_, v)| p(v)).map(|(_, v)| v).collect() }
    }
    struct MemRepo<K, V> { data: HashMap<K, V>, order: Vec<K> }
    impl<K: Eq + Hash + Clone + 'static, V: 'static> Repository<K, V> for MemRepo<K, V> {
        fn get(&self, k: &K) -> Option<&V> { self.data.get(k) }
        fn put(&mut self, k: K, v: V) -> Option<V> { if !self.data.contains_key(&k) { self.order.push(k.clone()); } self.data.insert(k, v) }
        fn all(&self) -> Box<dyn Iterator<Item = (&K, &V)> + '_> { Box::new(self.order.iter().map(move |k| (k, &self.data[k]))) }
    }
    struct Service<R> { repo: R, log: Vec<String> }
    impl<R: Repository<u32, User>> Service<R> {
        fn register(&mut self, id: u32, name: &str) -> Result<(), String> {
            if self.repo.get(&id).is_some() { return Err(format!("dup {}", id)); }
            self.repo.put(id, User { name: name.into(), age: 0 });
            self.log.push(format!("reg {}", id));
            Ok(())
        }
        fn birthday(&mut self, id: u32) -> Option<u8> {
            let mut u = self.repo.get(&id)?.clone();
            u.age += 1;
            let a = u.age;
            self.repo.put(id, u);
            Some(a)
        }
    }
    #[derive(Debug, Clone)] struct User { name: String, age: u8 }
    fn dump<K: Debug + 'static, V: Debug + 'static>(r: &dyn Repository<K, V>) -> Vec<String> { r.all().map(|(k, v)| format!("{:?}={:?}", k, v)).collect() }
    pub fn main() {
        let mut s = Service { repo: MemRepo { data: HashMap::new(), order: vec![] }, log: vec![] };
        println!("{:?} {:?} {:?}", s.register(2, "bo"), s.register(1, "al"), s.register(2, "x"));
        println!("{:?} {:?} {:?}", s.birthday(1), s.birthday(1), s.birthday(9));
        println!("{:?}", dump(&s.repo));
        println!("{:?}", s.repo.find(|u| u.age > 0).iter().map(|u| &u.name).collect::<Vec<_>>());
        println!("{:?}", s.log);
    }
}
mod l02 {
    use std::marker::PhantomData;
    use std::fmt;
    struct Missing; struct Present<T>(T);
    struct ReqBuilder<U, M> { url: U, method: M, headers: Vec<(String, String)> }
    impl ReqBuilder<Missing, Missing> { fn new() -> Self { ReqBuilder { url: Missing, method: Missing, headers: vec![] } } }
    impl<M> ReqBuilder<Missing, M> { fn url(self, u: impl Into<String>) -> ReqBuilder<Present<String>, M> { ReqBuilder { url: Present(u.into()), method: self.method, headers: self.headers } } }
    impl<U> ReqBuilder<U, Missing> { fn method(self, m: &'static str) -> ReqBuilder<U, Present<&'static str>> { ReqBuilder { url: self.url, method: Present(m), headers: self.headers } } }
    impl<U, M> ReqBuilder<U, M> { fn header(mut self, k: &str, v: impl ToString) -> Self { self.headers.push((k.into(), v.to_string())); self } }
    impl ReqBuilder<Present<String>, Present<&'static str>> { fn build(self) -> Req { Req { line: format!("{} {}", self.method.0, self.url.0), headers: self.headers } } }
    struct Req { line: String, headers: Vec<(String, String)> }
    impl fmt::Display for Req { fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result { writeln!(f, "{}", self.line)?; for (k, v) in &self.headers { writeln!(f, "{}: {}", k, v)?; } Ok(()) } }
    struct Id<T> { raw: u64, _t: PhantomData<fn() -> T> }
    impl<T> Clone for Id<T> { fn clone(&self) -> Self { *self } }
    impl<T> Copy for Id<T> {}
    impl<T> PartialEq for Id<T> { fn eq(&self, o: &Self) -> bool { self.raw == o.raw } }
    impl<T> fmt::Debug for Id<T> { fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result { write!(f, "Id<{}>({})", std::any::type_name::<T>().rsplit("::").next().unwrap(), self.raw) } }
    struct Order; struct Customer;
    pub fn main() {
        let r = ReqBuilder::new().header("a", 1).method("GET").header("b", true).url("/x").build();
        print!("{}", r);
        let r2 = ReqBuilder::new().url(String::from("/y")).method("POST").build();
        print!("{}", r2);
        let o: Id<Order> = Id { raw: 7, _t: PhantomData };
        let c: Id<Customer> = Id { raw: 7, _t: PhantomData };
        let o2 = o;
        println!("{:?} {:?} {}", o, c, o == o2);
    }
}
mod x26 {
    const A: u8 = 300.5f64 as u8;
    const B: i32 = f64::NAN as i32;
    const C: i64 = 1e30f64 as i64;
    const D: u32 = -5.5f32 as u32;
    const E: i8 = -1e10 as i8;
    const F: u128 = 1e40f64 as u128;
    const G: f32 = 16777217u32 as f32;
    const H: f32 = u128::MAX as f32;
    const I: f64 = u64::MAX as f64;
    const J: u64 = (u64::MAX as f64) as u64;
    const K: i16 = (-32768.9f64) as i16;
    const L: usize = 2.9f64 as usize;
    const M: [u8; 3] = [255.0f32 as u8, 256.0f32 as u8, -0.0f32 as u8];
    const N: f32 = 0.1f64 as f32;
    const O: f64 = 1.0f64 / 3.0;
    const P: f32 = (1.0f32 / 3.0) * 3.0;
    const Q: f64 = 0.1 + 0.2;
    const R: i32 = (2.5f64).round() as i32 * 10 + (-2.5f64 as i32);
    const S: u8 = (-1i32) as u8;
    const T: i64 = i64::MIN / 3 % 7;
    const U: f64 = 1e308 * 10.0;
    const V: u32 = 5.0e9 as u32;
    static ARR: [f32; 2] = [1.0 / 7.0, 2.0f32 * 1.1];
    pub fn main() {
        println!("{} {} {} {} {} {}", A, B, C, D, E, F);
        println!("{:?} {:?} {:?} {} {} {}", G, H, I, J, K, L);
        println!("{:?} {:?} {:?} {:?} {:?} {}", M, N, O, P, Q, R);
        println!("{} {} {} {} {:?}", S, T, U, V, ARR);
        let arr = [0u8; 2.5f64 as usize];
        println!("{}", arr.len());
    }
}
mod x27 {
    use std::future::Future;
    use std::pin::pin;
    use std::task::{Context, Poll, Waker};
    fn block<F: Future>(f: F) -> F::Output {
        let w = Waker::noop();
        let mut cx = Context::from_waker(&w);
        let mut f = pin!(f);
        loop { if let Poll::Ready(v) = f.as_mut().poll(&mut cx) { return v; } }
    }
    async fn apply<F: AsyncFn(u32) -> u32>(f: F, x: u32) -> u32 { f(x).await + f(x + 1).await }
    async fn apply_mut(mut f: impl AsyncFnMut(&str)) { f("a").await; f("bb").await; }
    async fn apply_once<F>(f: F) -> String where F: AsyncFnOnce() -> String { f().await }
    pub fn main() {
        let k = 10;
        let add = async |x: u32| x + k;
        println!("{}", block(apply(add, 1)));
        let mut seen = Vec::new();
        block(apply_mut(async |s: &str| { seen.push(s.len()); }));
        println!("{:?}", seen);
        let owned = String::from("own");
        println!("{}", block(apply_once(async move || owned + "ed")));
        let c = async || 5u8;
        println!("{}", block(c()));
        let futs: Vec<std::pin::Pin<Box<dyn Future<Output = u32>>>> = (0..3u32).map(|i| Box::pin(async move { i * i }) as std::pin::Pin<Box<dyn Future<Output = u32>>>).collect();
        let mut total = 0;
        for f in futs { total += block(f); }
        println!("{}", total);
    }
}
mod x28 {
    use std::ops::{Add, AddAssign, Mul, Neg, Sub};
    #[derive(Debug, Clone, PartialEq)]
    struct M<T> { d: Vec<T> }
    impl<'a, T: Add<Output = T> + Copy> Add<&'a M<T>> for &'a M<T> { type Output = M<T>; fn add(self, o: &M<T>) -> M<T> { M { d: self.d.iter().zip(&o.d).map(|(&a, &b)| a + b).collect() } } }
    impl<T: Add<Output = T> + Copy> Add for M<T> { type Output = M<T>; fn add(self, o: M<T>) -> M<T> { &self + &o } }
    impl<'a, T: Add<Output = T> + Copy> Add<&'a M<T>> for M<T> { type Output = M<T>; fn add(self, o: &M<T>) -> M<T> { &self + o } }
    impl<'a, T: AddAssign + Copy> AddAssign<&'a M<T>> for M<T> { fn add_assign(&mut self, o: &M<T>) { for (a, &b) in self.d.iter_mut().zip(&o.d) { *a += b; } } }
    impl<T: Neg<Output = T> + Copy> Neg for &M<T> { type Output = M<T>; fn neg(self) -> M<T> { M { d: self.d.iter().map(|&x| -x).collect() } } }
    impl<T: Mul<Output = T> + Copy> Mul<T> for &M<T> { type Output = M<T>; fn mul(self, k: T) -> M<T> { M { d: self.d.iter().map(|&x| x * k).collect() } } }
    impl<'a, T: Add<Output = T> + Copy + Default> std::iter::Sum<&'a M<T>> for M<T> where T: 'a { fn sum<I: Iterator<Item = &'a M<T>>>(mut it: I) -> M<T> { let first = it.next().cloned().unwrap_or(M { d: vec![] }); it.fold(first, |acc, m| acc + m) } }
    impl<T: Sub<Output = T> + Copy> Sub for M<T> { type Output = Self; fn sub(self, o: Self) -> Self { M { d: self.d.into_iter().zip(o.d).map(|(a, b)| a - b).collect() } } }
    #[derive(Debug, PartialEq, PartialOrd)] struct Name(String);
    impl PartialEq<str> for Name { fn eq(&self, o: &str) -> bool { self.0 == o } }
    impl PartialEq<&str> for Name { fn eq(&self, o: &&str) -> bool { self.0 == *o } }
    impl PartialEq<Name> for str { fn eq(&self, o: &Name) -> bool { self == o.0 } }
    impl PartialEq<Name> for &str { fn eq(&self, o: &Name) -> bool { *self == o.0 } }
    pub fn main() {
        let a = M { d: vec![1, 2, 3] };
        let b = M { d: vec![10, 20, 30] };
        let c = &a + &b;
        let d = c.clone() + &a;
        let mut e = d.clone() + c.clone();
        e += &a;
        println!("{:?} {:?} {:?}", c, d, e);
        println!("{:?} {:?}", -&a, &b * 2);
        let s: M<i32> = [a.clone(), b.clone(), c.clone()].iter().sum();
        println!("{:?} {:?}", s, b.clone() - a.clone());
        let f = M { d: vec![0.5, 1.5] };
        println!("{:?}", &(&f + &f) * 2.0);
        let n = Name("x".into());
        println!("{} {} {} {}", n == *"x", n == "x", *"x" == n, "y" == n);
        println!("{} {}", &n < &Name("y".into()), Name("b".into()) > Name("a".into()));
        let names = vec![Name("q".into()), Name("x".into())];
        println!("{:?}", names.iter().position(|m| *m == "x"));
        println!("{}", names.contains(&Name("q".into())));
    }
}
mod x46 {
    #[derive(Debug, Default, Clone, Copy, PartialEq)]
    enum Mode { Fast, #[default] Normal, Slow }
    #[derive(Debug, Default)]
    struct Cfg { mode: Mode, level: u8, name: String, tags: Vec<&'static str>, ratio: f32 }
    fn bucket(n: i32) -> &'static str {
        match n {
            i32::MIN..0 => "neg",
            0 => "zero",
            1..10 => "small",
            x @ 10..=99 if x % 2 == 0 => "even-mid",
            10..=99 => "odd-mid",
            100.. => "big",
        }
    }
    const fn square(x: u32) -> u32 { x * x }
    pub fn main() {
        let c = Cfg::default();
        println!("{:?}", c);
        let c2 = Cfg { level: 3, ..Default::default() };
        println!("{:?} {}", c2.mode, c2.level);
        for n in [-5, 0, 7, 42, 43, 100, 1000] { print!("{} ", bucket(n)); }
        println!();
        let arr = [const { square(3) }; 3];
        let v: Vec<Option<String>> = vec![const { None }; 2];
        println!("{:?} {:?}", arr, v);
        let cs = c"hello";
        println!("{:?} {}", cs, cs.to_bytes().len());
        let raw = r#"a "quoted" \n str"#;
        let bs = b"bytes\x00\xff";
        let rbs = br"raw\bytes";
        println!("{} {:?} {:?}", raw, bs, rbs);
        let ch = '\u{1F600}';
        println!("{} {}", ch.len_utf8(), '\x41');
        let x = 5;
        let r = 'outer: {
            if x > 3 { break 'outer "big"; }
            "small"
        };
        println!("{}", r);
        let tuple = (1, (2, [3, 4, 5]));
        let (a, (b, [c, rest @ ..])) = tuple;
        println!("{} {} {} {:?}", a, b, c, rest);
        let num = 1_000_000u64;
        let hex = 0xdead_beefu32;
        let oct = 0o777;
        let bin = 0b1010_1010u8;
        println!("{} {} {} {} {}", num, hex, oct, bin, 1e3);
        let nested = Some(Some(3));
        if let Some(Some(n @ 1..=5)) = nested { println!("nested {}", n); }
        let s = match "hello" { s if s.len() > 3 => s.to_uppercase(), s => s.to_string() };
        println!("{}", s);
        #[allow(unused_labels)]
        'l: for i in 0..3 { if i == 1 { continue; } println!("i={}", i); }
        let closure_ret = || -> Result<u8, String> { let v: u8 = "7".parse().map_err(|_| "e")?; Ok(v) };
        println!("{:?}", closure_ret());
        let m = Mode::Slow;
        println!("{}", matches!(m, Mode::Slow | Mode::Fast));
        use Mode::*;
        println!("{:?}", [Fast, Normal, Slow].iter().position(|&k| k == Normal));
    }
}
mod x47 {
    use std::hint::black_box as bb;
    use std::panic::catch_unwind as cu;
    pub fn main() {
        println!("{} {} {}", 1u32.wrapping_shl(33), 1i8.wrapping_shl(9), (-128i8).wrapping_shr(9));
        println!("{:?} {:?} {}", 1u64.checked_shl(u32::MAX), 1u64.checked_shr(63), 0x80u8.rotate_left(200));
        println!("{} {}", 1u128.wrapping_shl(130), u128::MAX.rotate_right(127));
        println!("{:?} {:?}", 3u8.overflowing_shl(8), (-1i32).overflowing_shr(32));
        println!("{}", cu(|| bb(i64::MIN) % bb(-1)).is_err());
        println!("{}", cu(|| bb(i64::MIN).wrapping_rem(-1)).unwrap());
        println!("{}", cu(|| bb(1u8) << bb(8u32)).is_err());
        println!("{}", cu(|| bb(1i16) >> bb(-1i32)).is_err());
        println!("{}", cu(|| bb(255u8) as i8 as u8 + bb(1)).is_err());
        println!("{}", cu(|| bb(i8::MIN).abs()).is_err());
        println!("{}", cu(|| bb(-5i32).pow(bb(31))).is_err());
        println!("{}", cu(|| bb(2u64).pow(bb(64))).is_err());
        println!("{}", cu(|| (bb(usize::MAX) as u128 + 1) as usize).unwrap());
        println!("{}", cu(|| bb(i32::MAX).isqrt()).unwrap());
        println!("{}", cu(|| bb(-1i32).isqrt()).is_err());
        println!("{}", cu(|| bb(0u32).ilog2()).is_err());
        println!("{}", cu(|| bb(10u32).div_ceil(bb(0))).is_err());
        println!("{}", cu(|| bb(-7i32).div_euclid(bb(0))).is_err());
        println!("{:?}", cu(|| bb(i32::MIN).checked_div_euclid(-1)).unwrap());
        println!("{}", cu(|| { let mut x = bb(250u8); x += 10; x }).is_err());
        println!("{}", cu(|| { let mut x = bb(1u32); x <<= bb(32); x }).is_err());
        println!("{}", cu(|| { let mut x = bb(-128i8); x *= -1; x }).is_err());
        println!("{}", cu(|| bb(3usize) - bb(4)).is_err());
        println!("{}", cu(|| bb([1u8, 2])[bb(2)]).is_err());
        println!("{}", cu(|| { let v: Vec<u8> = Vec::with_capacity(bb(4)); v[0] }).is_err());
        println!("{}", cu(|| bb("abc").chars().nth(5).unwrap()).is_err());
        println!("{}", cu(|| u8::try_from(bb(300u16)).unwrap()).is_err());
        println!("{}", cu(|| char::from_u32(bb(0xD800)).unwrap()).is_err());
    }
}
fn main() {
    println!("== l01");
    l01::main();
    println!("== l02");
    l02::main();
    println!("== x26");
    x26::main();
    println!("== x27");
    x27::main();
    println!("== x28");
    x28::main();
    println!("== x46");
    x46::main();
    println!("== x47");
    x47::main();
}
