mod s01 {
    use std::rc::Rc;
    use std::pin::Pin;
    struct Job { name: String }
    impl Job {
        fn consume(self: Box<Self>) -> String { format!("boxed {}", self.name) }
        fn share(self: Rc<Self>) -> usize { Rc::strong_count(&self) }
        fn pinned(self: Pin<&mut Self>) -> usize { self.name.len() }
        fn by_ref(self: &Self) -> &str { &self.name }
    }
    trait Run { fn run(self: Box<Self>) -> i32; fn peek(&self) -> i32; }
    struct R(i32);
    impl Run for R { fn run(self: Box<Self>) -> i32 { self.0 * 2 } fn peek(&self) -> i32 { self.0 } }
    pub fn main() {
        let b = Box::new(Job { name: "a".into() });
        println!("{}", b.by_ref());
        println!("{}", b.consume());
        let r = Rc::new(Job { name: "bb".into() });
        let r2 = r.clone();
        println!("{} {}", r.share(), Rc::strong_count(&r2));
        let mut j = Job { name: "ccc".into() };
        println!("{}", Pin::new(&mut j).pinned());
        let tasks: Vec<Box<dyn Run>> = vec![Box::new(R(3)), Box::new(R(4))];
        let peeks: Vec<i32> = tasks.iter().map(|t| t.peek()).collect();
        let outs: Vec<i32> = tasks.into_iter().map(|t| t.run()).collect();
        println!("{:?} {:?}", peeks, outs);
    }
}

mod s02 {
    trait Describe { fn describe(&self) -> String; }
    impl Describe for i32 { fn describe(&self) -> String { format!("i32 {}", self) } }
    impl<T: Describe + ?Sized> Describe for &T { fn describe(&self) -> String { format!("&{}", (**self).describe()) } }
    impl<T: Describe> Describe for Box<T> { fn describe(&self) -> String { format!("box({})", (**self).describe()) } }
    impl<T: Describe> Describe for Vec<T> { fn describe(&self) -> String { format!("[{}]", self.iter().map(|x| x.describe()).collect::<Vec<_>>().join(", ")) } }
    impl<T: Describe> Describe for Option<T> { fn describe(&self) -> String { match self { Some(x) => format!("some {}", x.describe()), None => "none".into() } } }
    impl Describe for str { fn describe(&self) -> String { format!("str {}", self) } }
    impl<A: Describe, B: Describe> Describe for (A, B) { fn describe(&self) -> String { format!("({} & {})", self.0.describe(), self.1.describe()) } }
    fn show<T: Describe + ?Sized>(t: &T) -> String { t.describe() }
    struct Wrapper(i32);
    impl Wrapper { fn describe(&self) -> String { "inherent".into() } }
    impl Describe for Wrapper { fn describe(&self) -> String { "trait".into() } }
    pub fn main() {
        let x = 5;
        println!("{}", x.describe());
        println!("{}", (&x).describe());
        println!("{}", (&&x).describe());
        println!("{}", show(&&x));
        println!("{}", Box::new(7).describe());
        println!("{}", vec![Some(1), None].describe());
        println!("{}", "hi".describe());
        println!("{}", show("yo"));
        println!("{}", (1, vec![2]).describe());
        let w = Wrapper(0);
        println!("{} {} {}", w.describe(), Describe::describe(&w), <Wrapper as Describe>::describe(&w));
        let d: &dyn Describe = &vec![3, 4];
        println!("{}", d.describe());
    }
}

mod s03 {
    struct Countdown { n: u32, lo: u32 }
    impl Iterator for Countdown {
        type Item = u32;
        fn next(&mut self) -> Option<u32> { if self.n > self.lo { self.n -= 1; Some(self.n + 1) } else { None } }
        fn size_hint(&self) -> (usize, Option<usize>) { let k = (self.n - self.lo) as usize; (k, Some(k)) }
    }
    impl DoubleEndedIterator for Countdown {
        fn next_back(&mut self) -> Option<u32> { if self.n > self.lo { self.lo += 1; Some(self.lo) } else { None } }
    }
    impl ExactSizeIterator for Countdown {}
    impl std::iter::FusedIterator for Countdown {}
    struct Fib { a: u64, b: u64 }
    impl Iterator for Fib { type Item = u64; fn next(&mut self) -> Option<u64> { let r = self.a; self.a = self.b; self.b += r; Some(r) } }
    pub fn main() {
        let c = Countdown { n: 5, lo: 0 };
        println!("{} {:?}", c.len(), c.rev().collect::<Vec<_>>());
        let mut c = Countdown { n: 6, lo: 0 };
        println!("{:?} {:?} {:?} {}", c.next(), c.next_back(), c.nth(1), c.len());
        println!("{:?}", c.collect::<Vec<_>>());
        let z: Vec<_> = Countdown { n: 3, lo: 0 }.zip(Countdown { n: 10, lo: 0 }.rev()).collect();
        println!("{:?}", z);
        println!("{:?}", Fib { a: 0, b: 1 }.skip(10).step_by(5).take(3).collect::<Vec<_>>());
        println!("{:?}", Fib { a: 0, b: 1 }.take_while(|&x| x < 100).filter(|x| x % 2 == 0).collect::<Vec<_>>());
        println!("{:?}", Fib { a: 0, b: 1 }.position(|x| x > 1000));
        println!("{}", Countdown { n: 4, lo: 0 }.map(|x| x * x).rev().fold(String::new(), |s, x| s + &x.to_string() + ","));
        let mut pk = Countdown { n: 3, lo: 0 }.peekable();
        if let Some(p) = pk.peek_mut() { *p *= 100; }
        println!("{:?}", pk.collect::<Vec<_>>());
        println!("{:?}", Countdown { n: 5, lo: 0 }.rev().skip(1).rev().collect::<Vec<_>>());
        println!("{:?}", Countdown { n: 5, lo: 0 }.rposition(|x| x == 4));
        println!("{:?}", Countdown { n: 8, lo: 2 }.rfind(|x| x % 3 == 0));
        println!("{:?}", Countdown { n: 8, lo: 2 }.rfold(0, |a, x| a * 10 + x));
        let v: Vec<u32> = Countdown { n: 3, lo: 0 }.chain(Countdown { n: 2, lo: 0 }).rev().collect();
        println!("{:?}", v);
        let (s, l) = Countdown { n: 4, lo: 0 }.fold((0, 0), |(s, l), x| (s + x, l + 1));
        println!("{} {}", s, l);
        println!("{:?}", Countdown { n: 4, lo: 0 }.max_by_key(|x| (*x as i32 - 2).abs()));
        println!("{:?}", Countdown { n: 4, lo: 0 }.min_by_key(|x| (*x as i32 - 2).abs()));
        println!("{}", Countdown { n: 10, lo: 0 }.flat_map(|x| 0..x).count());
        println!("{:?}", Countdown { n: 4, lo: 0 }.map(|x| x as f64 / 2.0).fold(f64::NEG_INFINITY, f64::max));
        println!("{}", Countdown { n: 4, lo: 0 }.eq([4, 3, 2, 1]));
        println!("{}", Countdown { n: 4, lo: 0 }.lt([4, 3, 3].into_iter()));
    }
}

mod s04 {
    use std::marker::PhantomData;
    #[derive(Debug, Clone, Copy, PartialEq)]
    struct Meters;
    #[derive(Debug, Clone, Copy, PartialEq)]
    struct Feet;
    #[derive(Debug, Clone, Copy, PartialEq)]
    struct Len<U> { v: f64, _u: PhantomData<U> }
    impl<U> Len<U> { fn new(v: f64) -> Self { Len { v, _u: PhantomData } } }
    impl<U> std::ops::Add for Len<U> { type Output = Len<U>; fn add(self, o: Self) -> Self { Len::new(self.v + o.v) } }
    impl From<Len<Feet>> for Len<Meters> { fn from(f: Len<Feet>) -> Self { Len::new(f.v * 0.3048) } }
    struct Open; struct Closed;
    struct Door<S> { name: String, _s: PhantomData<S> }
    impl Door<Closed> { fn open(self) -> Door<Open> { println!("opening {}", self.name); Door { name: self.name, _s: PhantomData } } }
    impl Door<Open> { fn close(self) -> Door<Closed> { println!("closing {}", self.name); Door { name: self.name, _s: PhantomData } } fn walk(&self) { println!("walk through {}", self.name) } }
    #[derive(Default, Debug, Clone)]
    struct Holder<T: Default + Clone> { items: Vec<T>, extra: T }
    trait Zero { fn zero() -> Self; fn is_zero(&self) -> bool; }
    impl Zero for i64 { fn zero() -> Self { 0 } fn is_zero(&self) -> bool { *self == 0 } }
    impl Zero for f32 { fn zero() -> Self { 0.0 } fn is_zero(&self) -> bool { *self == 0.0 } }
    fn count_zeros<T: Zero>(v: &[T]) -> usize { v.iter().filter(|x| x.is_zero()).count() + T::zero().is_zero() as usize }
    pub fn main() {
        let a: Len<Meters> = Len::new(1.0);
        let f: Len<Feet> = Len::new(10.0);
        let total = a + f.into();
        println!("{:.4}", total.v);
        let d = Door::<Closed> { name: "front".into(), _s: PhantomData };
        let d = d.open();
        d.walk();
        let _d = d.close();
        let h: Holder<String> = Holder::default();
        let h2 = Holder { items: vec![1u8], extra: 2 };
        println!("{:?} {:?}", h, h2.clone());
        println!("{} {}", count_zeros(&[0i64, 1, 0]), count_zeros::<f32>(&[]));
        println!("{}", std::mem::size_of::<Len<Feet>>());
    }
}

mod s05 {
    use std::collections::HashMap;
    struct Interner { map: HashMap<String, usize>, names: Vec<String> }
    impl Interner {
        fn intern(&mut self, s: &str) -> usize {
            if let Some(&i) = self.map.get(s) { return i; }
            let i = self.names.len();
            self.names.push(s.to_string());
            self.map.insert(s.to_string(), i);
            i
        }
        fn lookup(&self, i: usize) -> &str { &self.names[i] }
    }
    pub fn main() {
        let mut v = vec![1, 2];
        v.push(v.len());
        v.insert(0, v[2] * 10);
        let n = v.len();
        v[n - 1] += v[0];
        println!("{:?}", v);
        let mut m: HashMap<&str, Vec<usize>> = HashMap::new();
        let ml = m.len();
        m.entry("a").or_default().push(ml);
        println!("{:?}", m.get("a"));
        let mut it = Interner { map: HashMap::new(), names: vec![] };
        let ids: Vec<usize> = "x y x z y".split(' ').map(|s| it.intern(s)).collect();
        println!("{:?} {}", ids, it.lookup(2));
        let mut s = String::from("ab");
        s.push_str(&s.clone());
        s.truncate(s.len() - 1);
        println!("{}", s);
        let mut grid = vec![vec![0; 3]; 3];
        for i in 0..3 { for j in 0..3 { grid[i][j] = if i > 0 { grid[i - 1][j] + j } else { j }; } }
        println!("{:?}", grid);
        let (a, b) = grid.split_at_mut(1);
        a[0][0] = b[1][2];
        std::mem::swap(&mut a[0][1], &mut b[0][1]);
        println!("{:?}", grid);
        let mut nums = vec![3, 1, 2];
        let mx = *nums.iter().max().unwrap();
        nums.retain(|&x| x != mx);
        println!("{:?}", nums);
    }
}

mod s06 {
    use std::fmt::Debug;
    trait Animal { fn name(&self) -> String; fn speak(&self) -> String { format!("{} makes a sound", self.name()) } }
    trait Loud: Animal { fn shout(&self) -> String { self.speak().to_uppercase() } }
    #[derive(Debug)] struct Cat;
    impl Animal for Cat { fn name(&self) -> String { "cat".into() } fn speak(&self) -> String { "meow".into() } }
    impl Loud for Cat {}
    fn as_animal(l: &dyn Loud) -> &dyn Animal { l }
    fn all_shout<T: Loud + Debug>(xs: &[T]) -> Vec<String> { xs.iter().map(|x| format!("{:?}:{}", x, x.shout())).collect() }
    fn boxed_mut(b: &mut dyn Loud) -> &mut dyn Animal { b }
    fn opt_upcast(o: Option<&dyn Loud>) -> Option<&dyn Animal> { o.map(|l| l as &dyn Animal) }
    pub fn main() {
        let c = Cat;
        let l: &dyn Loud = &c;
        println!("{} {}", l.shout(), as_animal(l).speak());
        println!("{:?}", all_shout(&[Cat, Cat]));
        let mut c2 = Cat;
        println!("{}", boxed_mut(&mut c2).name());
        println!("{:?}", opt_upcast(Some(&c)).map(|a| a.name()));
        let arc: std::sync::Arc<dyn Loud + Send + Sync> = std::sync::Arc::new(Cat);
        let arc2: std::sync::Arc<dyn Animal + Send + Sync> = arc;
        println!("{}", arc2.speak());
        let bs: Vec<Box<dyn Loud>> = vec![Box::new(Cat)];
        let ba: Vec<Box<dyn Animal>> = bs.into_iter().map(|b| b as Box<dyn Animal>).collect();
        println!("{}", ba[0].name());
    }
}

fn main() {
    println!("== s01");
    s01::main();
    println!("== s02");
    s02::main();
    println!("== s03");
    s03::main();
    println!("== s04");
    s04::main();
    println!("== s05");
    s05::main();
    println!("== s06");
    s06::main();
}
