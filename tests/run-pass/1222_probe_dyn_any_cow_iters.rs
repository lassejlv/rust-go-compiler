mod p01 {
    trait Shape { fn area(&self) -> f64; }
    impl dyn Shape {
        fn doubled(&self) -> f64 { self.area() * 2.0 }
    }
    struct Sq(f64);
    impl Shape for Sq { fn area(&self) -> f64 { self.0 * self.0 } }
    impl PartialEq for dyn Shape {
        fn eq(&self, o: &dyn Shape) -> bool { self.area() == o.area() }
    }
    impl std::fmt::Debug for dyn Shape {
        fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result { write!(f, "Shape({})", self.area()) }
    }
    pub fn main() {
        let a: Box<dyn Shape> = Box::new(Sq(2.0));
        let b: Box<dyn Shape> = Box::new(Sq(2.0));
        println!("{} {} {:?}", a.doubled(), *a == *b, a);
        let v: Vec<Box<dyn Shape>> = vec![a, b, Box::new(Sq(1.0))];
        println!("{:?}", v);
    }
}

mod p02 {
    struct S { a: String, b: String }
    impl Drop for S { fn drop(&mut self) { println!("drop S"); } }
    struct T { a: String, b: Vec<i32> }
    pub fn main() {
        let t = T { a: "x".into(), b: vec![1] };
        let c = || println!("{}", t.a);
        let moved = t.b;
        c();
        println!("{:?}", moved);
        let mut t2 = T { a: "y".into(), b: vec![] };
        let mut push = || t2.b.push(3);
        let r = &t2.a;
        push();
        println!("{} {:?}", r, t2.b);
        let s = S { a: "1".into(), b: "2".into() };
        let c2 = move || s.a.len();
        println!("{}", c2());
    }
}

mod p03 {
    pub fn main() {
        let mut it = (1..10).into_iter();
        let first: Vec<i32> = it.by_ref().take(3).collect();
        let rest: i32 = it.sum();
        println!("{:?} {}", first, rest);
        let mut chars = "hello world".chars();
        let w: String = chars.by_ref().take_while(|c| *c != ' ').collect();
        println!("{} [{}]", w, chars.as_str());
        let v = vec![1, 2, 3];
        for (i, x) in v.iter().enumerate().rev() { print!("{}{} ", i, x); }
        println!();
        for (a, b) in std::iter::zip([1, 2], ["a", "b"]) { print!("{}{} ", a, b); }
        println!();
        let arr = [String::from("x"), String::from("y")];
        let v2: Vec<String> = arr.into_iter().rev().collect();
        println!("{:?}", v2);
        let mut sl = [5, 4, 3].iter();
        println!("{:?} {:?}", sl.as_slice(), sl.next_back());
        let s: std::collections::HashSet<i32> = [1, 2, 3].into_iter().collect();
        let mut sv: Vec<_> = s.into_iter().collect(); sv.sort();
        println!("{:?}", sv);
    }
}

mod p04 {
    #[derive(Debug, PartialEq, PartialOrd, Clone, Copy)]
    enum Lvl { Low = 1, Mid = 5, High = 10 }
    #[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
    enum Ev { A(i32), B { x: u8 }, C }
    pub fn main() {
        println!("{} {} {:?}", Lvl::Low < Lvl::High, Lvl::Mid as i32, Lvl::High.partial_cmp(&Lvl::Mid));
        let mut v = vec![Ev::C, Ev::B { x: 2 }, Ev::A(5), Ev::A(-1), Ev::B { x: 1 }];
        v.sort();
        println!("{:?}", v);
        println!("{:?}", v.iter().max());
        #[repr(u8)]
        #[derive(Debug, Clone, Copy)]
        enum Small { X = 200, Y }
        println!("{} {} {}", Small::X as u8, Small::Y as u8, Small::Y as i8);
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
        enum Neg { A = -3, B, C = 7 }
        println!("{} {} {}", Neg::A as i32, Neg::B as i64, Neg::C as u8);
        println!("{}", std::mem::size_of::<Small>());
    }
}

mod p05 {
    use std::any::Any;
    fn describe(x: &dyn Any) -> String {
        if let Some(i) = x.downcast_ref::<i32>() { format!("i32 {}", i) }
        else if let Some(s) = x.downcast_ref::<String>() { format!("string {}", s) }
        else if x.is::<&str>() { "str".to_string() }
        else { "unknown".to_string() }
    }
    pub fn main() {
        let items: Vec<Box<dyn Any>> = vec![Box::new(5i32), Box::new(String::from("hi")), Box::new("s"), Box::new(1.5f64)];
        for i in &items { println!("{}", describe(i.as_ref())); }
        let b: Box<dyn Any> = Box::new(42u64);
        match b.downcast::<u64>() { Ok(v) => println!("u64 {}", v), Err(_) => println!("no") }
        let mut m: Box<dyn Any> = Box::new(vec![1]);
        if let Some(v) = m.downcast_mut::<Vec<i32>>() { v.push(2); }
        println!("{:?}", m.downcast_ref::<Vec<i32>>());
        println!("{:?}", std::any::type_name::<Option<i32>>());
        println!("{}", std::any::TypeId::of::<i32>() == std::any::TypeId::of::<i32>());
    }
}

mod p09 {
    use std::collections::HashMap;
    #[derive(Debug, Hash, PartialEq, Eq, Clone)]
    enum K { A(u8), B(String) }
    pub fn main() {
        let mut m: HashMap<K, Vec<i32>> = HashMap::new();
        m.entry(K::A(1)).or_default().push(1);
        m.entry(K::B("x".into())).or_insert_with(Vec::new).push(2);
        m.entry(K::A(1)).or_default().push(3);
        let mut ks: Vec<_> = m.iter().map(|(k, v)| format!("{:?}={:?}", k, v)).collect();
        ks.sort();
        println!("{:?}", ks);
        println!("{:?}", m.get(&K::A(1)));
        println!("{:?}", m.remove(&K::B("x".into())));
        println!("{}", m.len());
        let words = "a b a c b a";
        let mut freq: HashMap<&str, i32> = HashMap::new();
        for w in words.split(' ') { *freq.entry(w).or_insert(0) += 1; }
        let mut fv: Vec<_> = freq.into_iter().collect();
        fv.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(b.0)));
        println!("{:?}", fv);
        let hm: HashMap<i32, i32> = (0..5).map(|i| (i, i * i)).collect();
        let mut total: Vec<_> = hm.values().copied().collect(); total.sort();
        println!("{:?} {}", total, hm.contains_key(&3));
    }
}

mod p10 {
    pub fn main() {
        let mut x = 0u8;
        for _ in 0..300 { x = x.wrapping_add(1); }
        println!("{}", x);
        let y: u8 = 200;
        let z = y.checked_add(100).unwrap_or(0);
        println!("{}", z);
        let w = (y as u16 + 100) as u8;
        println!("{}", w);
        let shift = 9u32;
        println!("{:?}", 1u8.checked_shl(shift));
        let a = i32::MAX;
        let r = std::panic::catch_unwind(|| a + std::hint::black_box(1));
        println!("{}", r.is_err());
        let r = std::panic::catch_unwind(|| 1u32 << std::hint::black_box(33));
        println!("{}", r.is_err());
        let r = std::panic::catch_unwind(|| i32::MIN / std::hint::black_box(-1));
        println!("{}", r.is_err());
        let r = std::panic::catch_unwind(|| -std::hint::black_box(i64::MIN));
        println!("{}", r.is_err());
        let r = std::panic::catch_unwind(|| std::hint::black_box(0u32) - 1);
        println!("{}", r.is_err());
        let r = std::panic::catch_unwind(|| std::hint::black_box(10u64).pow(20));
        println!("{}", r.is_err());
        let r = std::panic::catch_unwind(|| std::hint::black_box(5i8).abs() * 30);
        println!("{}", r.is_err());
    }
}

mod p11 {
    use std::borrow::Cow;
    fn normalize(s: &str) -> Cow<str> {
        if s.contains(' ') { Cow::Owned(s.replace(' ', "_")) } else { Cow::Borrowed(s) }
    }
    pub fn main() {
        for s in ["ab", "a b"] {
            let n = normalize(s);
            match &n { Cow::Borrowed(_) => print!("B "), Cow::Owned(_) => print!("O ") }
            println!("{} {}", n, n.len());
        }
        let mut c: Cow<[i32]> = Cow::Borrowed(&[1, 2]);
        c.to_mut().push(3);
        println!("{:?}", c);
        let o: String = normalize("x y").into_owned();
        println!("{}", o);
        let c2: Cow<str> = "lit".into();
        let c3: Cow<str> = String::from("own").into();
        println!("{}{}", c2, c3 + "ed");
    }
}

mod p12 {
    use std::fmt::Display;
    fn show_all(items: &[&dyn Display]) -> String {
        items.iter().map(|d| d.to_string()).collect::<Vec<_>>().join(",")
    }
    fn longest<'a>(a: &'a str, b: &'a str) -> &'a str { if a.len() >= b.len() { a } else { b } }
    fn pick<T: PartialOrd + Copy>(xs: impl IntoIterator<Item = T>) -> Option<T> {
        let mut it = xs.into_iter();
        let mut best = it.next()?;
        for x in it { if x > best { best = x } }
        Some(best)
    }
    fn evens(limit: u32) -> impl Iterator<Item = u32> + Clone { (0..limit).filter(|x| x % 2 == 0) }
    fn boxed(rev: bool) -> Box<dyn Iterator<Item = u32>> { if rev { Box::new((0..4).rev()) } else { Box::new(0..4) } }
    pub fn main() {
        println!("{}", show_all(&[&1, &"two", &3.5, &'c']));
        println!("{}", longest("abc", "de"));
        println!("{:?} {:?} {:?}", pick(vec![3, 9, 2]), pick([1.5, -2.0]), pick(Vec::<u8>::new()));
        let e = evens(10);
        let e2 = e.clone();
        println!("{:?} {}", e.collect::<Vec<_>>(), e2.count());
        println!("{:?} {:?}", boxed(true).collect::<Vec<_>>(), boxed(false).map(|x| x * 2).collect::<Vec<_>>());
    }
}

fn main() {
    println!("== p01");
    p01::main();
    println!("== p02");
    p02::main();
    println!("== p03");
    p03::main();
    println!("== p04");
    p04::main();
    println!("== p05");
    p05::main();
    println!("== p09");
    p09::main();
    println!("== p10");
    p10::main();
    println!("== p11");
    p11::main();
    println!("== p12");
    p12::main();
}
