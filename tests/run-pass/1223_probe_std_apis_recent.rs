mod q01 {
    pub fn main() {
        let v = vec![1, 1, 2, 3, 3, 3, 4, 1];
        let g: Vec<&[i32]> = v.chunk_by(|a, b| a == b).collect();
        println!("{:?}", g);
        let g: Vec<&[i32]> = v.chunk_by(|a, b| a <= b).collect();
        println!("{:?}", g);
        let mut w = vec![1, 2, 3, 4, 5, 6, 7];
        let ex: Vec<i32> = w.extract_if(.., |x| *x % 3 == 0).collect();
        println!("{:?} {:?}", ex, w);
        println!("{:?}", "a\nb\nc".split_inclusive('\n').collect::<Vec<_>>());
        println!("{} {} {}", 17u32.div_ceil(5), 17u32.next_multiple_of(5), 99i32.isqrt());
        println!("{:?} {:?} {}", 1000u32.checked_ilog10(), 0u32.checked_ilog2(), 255u8.ilog(3));
        println!("{} {}", 7u8.midpoint(10), u8::MAX.midpoint(253));
        let a = [1u8, 2, 3, 4, 5];
        println!("{:?} {:?}", a.split_first_chunk::<2>(), a.last_chunk::<3>());
        println!("{:?}", a.first_chunk::<9>());
        let (ch, rem) = a.as_chunks::<2>();
        println!("{:?} {:?}", ch, rem);
        println!("{:?}", std::iter::repeat_n("x", 3).collect::<String>());
        println!("{:?}", [3, 1, 2].iter().is_sorted_by(|a, b| a <= b));
    }
}

mod q02 {
    pub fn main() {
        let mut o = Some(4);
        println!("{} {}", o.is_some_and(|x| x > 3), o.is_none_or(|x| x > 10));
        println!("{:?} {:?}", o.take_if(|x| *x % 2 == 1), o);
        println!("{:?} {:?}", o.take_if(|x| *x % 2 == 0), o);
        let mut n: Option<Vec<i32>> = None;
        n.get_or_insert_with(Vec::new).push(1);
        println!("{:?}", n);
        let s: Option<String> = Some("hey".into());
        println!("{:?} {:?}", s.as_deref(), s.as_ref().map(|x| x.len()));
        let r: Option<Result<i32, String>> = Some(Ok(3));
        println!("{:?}", r.transpose());
        println!("{:?} {:?}", Some(Some(1)).flatten(), Some(1).zip(Some('a')));
        println!("{:?} {:?}", Some((1, 'b')).unzip(), Some(1).xor(None::<i32>));
        println!("{:?} {:?}", Some(2).filter(|x| x % 2 == 1), None.or(Some(9)));
        println!("{:?}", Some(3).inspect(|x| println!("saw {}", x)).map(|x| x + 1));
        println!("{:?}", Some(5).ok_or("none"));
        println!("{:?}", None::<i32>.ok_or_else(|| format!("e{}", 1)));
        println!("{}", Some(2).map_or(0, |x| x * 3) + None.map_or_else(|| 100, |x: i32| x));
        let mut x = Some(1);
        let xr = x.replace(5); let xi = *x.insert(8);
        println!("{:?} {:?} {:?}", xr, xi, x);
        let res: Result<i32, String> = Err("bad".into());
        println!("{} {}", res.clone().is_err_and(|e| e.len() == 3), res.clone().is_ok_and(|v| v > 0));
        println!("{:?}", res.as_ref().inspect_err(|e| println!("err={}", e)).map_err(|e| e.len()));
        println!("{:?} {:?}", res.clone().ok(), res.clone().err());
        println!("{:?}", res.clone().or_else(|e| if e == "bad" { Ok::<i32, String>(0) } else { Err(e) }));
        println!("{:?}", Ok::<i32, String>(3).and_then(|v| if v > 2 { Err("big".to_string()) } else { Ok(v) }));
        println!("{}", res.clone().unwrap_or_default());
        let rr: Result<&i32, ()> = Ok(&5);
        println!("{:?} {:?}", rr.copied(), Ok::<Result<i32, ()>, ()>(Ok(1)).flatten_compat());
        for v in Ok::<i32, ()>(7).iter() { println!("iter {}", v); }
        println!("{:?}", Some(4).iter().chain(None.iter()).collect::<Vec<_>>());
    }
    trait FlatCompat<T, E> { fn flatten_compat(self) -> Result<T, E>; }
    impl<T, E> FlatCompat<T, E> for Result<Result<T, E>, E> { fn flatten_compat(self) -> Result<T, E> { self.and_then(|x| x) } }
}

mod q03 {
    use std::collections::*;
    pub fn main() {
        let s: Option<i32> = [Some(1), Some(2), Some(3)].into_iter().sum();
        let s2: Option<i32> = [Some(1), None].into_iter().sum();
        let p: Result<i32, String> = vec![Ok(2), Ok(5)].into_iter().product();
        println!("{:?} {:?} {:?}", s, s2, p);
        let o: Option<Vec<i32>> = vec![Some(1), Some(2)].into_iter().collect();
        let r: Result<String, String> = vec![Ok("a"), Ok("b")].into_iter().collect();
        let r2: Result<Vec<i32>, String> = vec![Ok(1), Err("x".to_string()), Err("y".to_string())].into_iter().collect();
        println!("{:?} {:?} {:?}", o, r, r2);
        let b: Box<[i32]> = (1..4).collect();
        let bs: Box<str> = "boxed".into();
        let rs: std::rc::Rc<[u8]> = vec![1, 2].into();
        let rstr: std::rc::Rc<str> = std::rc::Rc::from("rcs");
        println!("{:?} {} {:?} {} {}", b, bs, rs, rstr, b.len() + bs.len());
        let dq: VecDeque<char> = "abc".chars().collect();
        let bm: BTreeMap<usize, char> = "xyz".chars().enumerate().collect();
        let bs2: BTreeSet<&str> = "b a c a".split(' ').collect();
        println!("{:?} {:?} {:?}", dq, bm, bs2);
        let (v, hs): (Vec<i32>, BTreeSet<i32>) = [(1, 2), (3, 2)].into_iter().unzip();
        println!("{:?} {:?}", v, hs);
        let st: String = vec!["ab", "cd"].into_iter().collect();
        let st2: String = vec![String::from("x"), String::from("y")].iter().map(|s| s.as_str()).collect();
        let st3: String = ['a', 'b'].iter().collect();
        let st4: String = vec![std::borrow::Cow::Borrowed("c"), std::borrow::Cow::Owned("ow".to_string())].into_iter().collect();
        println!("{} {} {} {}", st, st2, st3, st4);
        let mut bm2: BTreeMap<&str, i32> = BTreeMap::new();
        bm2.extend(vec![("k", 1), ("j", 2)]);
        let mut v2 = vec![0];
        v2.extend(Some(5));
        v2.extend(&[6, 7]);
        println!("{:?} {:?}", bm2, v2);
        let flat: Vec<i32> = vec![Some(1), None, Some(3)].into_iter().flatten().collect();
        println!("{:?}", flat);
        let total: i32 = vec![Ok::<i32, ()>(1), Err(()), Ok(2)].into_iter().flatten().sum();
        println!("{}", total);
        let lens: Vec<usize> = ["a", "bcd"].iter().map(|s| s.len()).collect();
        let owned: Vec<String> = ["a", "b"].iter().map(ToString::to_string).collect();
        let owned2: Vec<String> = ["a", "b"].into_iter().map(String::from).collect();
        let up: Vec<String> = owned.iter().map(|s| s.to_uppercase()).collect();
        let digits: Vec<bool> = "a1".bytes().map(|b| b.is_ascii_digit()).collect();
        let cs: Vec<bool> = "a1".chars().map(char::is_alphabetic).collect();
        println!("{:?} {:?} {:?} {:?} {:?} {:?}", lens, owned, owned2, up, digits, cs);
        let sum_len: usize = owned.iter().map(String::len).sum();
        println!("{}", sum_len);
        let parsed: Vec<i64> = "1,2,3".split(',').map(str::parse).map(Result::unwrap).collect();
        println!("{:?}", parsed);
        let somes: Vec<Option<i32>> = vec![1, 2].into_iter().map(Some).collect();
        println!("{:?}", somes);
    }
}

mod q04 {
    use std::sync::{OnceLock, LazyLock, Mutex};
    use std::cell::{OnceCell, LazyCell};
    static CONFIG: OnceLock<String> = OnceLock::new();
    static TABLE: LazyLock<Vec<u32>> = LazyLock::new(|| { println!("init table"); (0..5).map(|x| x * x).collect() });
    static COUNTER: Mutex<u32> = Mutex::new(0);
    fn bump() -> u32 { let mut g = COUNTER.lock().unwrap(); *g += 1; *g }
    pub fn main() {
        println!("{:?}", CONFIG.get());
        CONFIG.set("cfg".to_string()).unwrap();
        println!("{:?} {:?}", CONFIG.get(), CONFIG.set("x".into()).is_err());
        println!("before");
        println!("{} {}", TABLE[3], TABLE.len());
        bump(); bump();
        println!("{}", bump());
        let c: OnceCell<i32> = OnceCell::new();
        println!("{} {}", c.get_or_init(|| 5), c.get_or_init(|| 6));
        let l = LazyCell::new(|| { println!("lazy"); 10 });
        println!("x");
        println!("{}", *l + 1);
        let v = std::mem::take(&mut vec![1, 2]);
        let mut s = String::from("a");
        let old = std::mem::replace(&mut s, "b".into());
        println!("{:?} {} {}", v, old, s);
        #[derive(Debug)] enum E { A(i32), B }
        println!("{}", std::mem::discriminant(&E::A(1)) == std::mem::discriminant(&E::A(2)));
        println!("{}", std::mem::discriminant(&E::A(1)) == std::mem::discriminant(&E::B));
        let mut a = 1; let mut b = 2;
        std::mem::swap(&mut a, &mut b);
        println!("{} {}", a, b);
    }
}

mod q05 {
    trait Base<T> { fn get(&self) -> T; }
    trait Derived<T: Clone>: Base<T> { fn twice(&self) -> (T, T) { (self.get(), self.get()) } }
    struct S(String);
    impl Base<String> for S { fn get(&self) -> String { self.0.clone() } }
    impl Derived<String> for S {}
    impl Base<u8> for S { fn get(&self) -> u8 { self.0.len() as u8 } }
    impl Derived<u8> for S {}
    fn apply<F>(f: F, s: &str) -> usize where F: for<'a> Fn(&'a str) -> &'a str { f(s).len() }
    fn call_twice<F: FnMut()>(mut f: F) { f(); f(); }
    pub fn main() {
        let s = S("abc".into());
        let a: (String, String) = s.twice();
        let b: (u8, u8) = Derived::<u8>::twice(&s);
        println!("{:?} {:?}", a, b);
        let d: &dyn Derived<u8> = &s;
        println!("{}", d.get());
        println!("{}", apply(|x| &x[1..], "hello"));
        println!("{}", apply(str::trim, "  hi  "));
        let mut n = 0;
        call_twice(|| n += 5);
        println!("{}", n);
        let fp: fn(&str) -> usize = str::len;
        let fs: Vec<fn(f64) -> f64> = vec![f64::sqrt, f64::abs, |x| x * 2.0];
        println!("{} {:?}", fp("four"), fs.iter().map(|f| f(-4.0f64.abs())).collect::<Vec<_>>());
        let m = [1, 2, 3].map(|x| x + 1);
        println!("{:?}", m);
    }
}

mod q06 {
    #[derive(Debug, PartialEq, PartialOrd, Clone, Copy, Default)]
    struct P { x: f64, y: f64 }
    #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
    struct Ver { major: u32, minor: u32, tag: Option<String> }
    pub fn main() {
        let mut ps = vec![P { x: 1.0, y: 2.0 }, P { x: 1.0, y: f64::NAN }, P { x: 0.5, y: 9.0 }];
        println!("{:?}", ps[0].partial_cmp(&ps[1]));
        println!("{:?}", ps[0].partial_cmp(&ps[2]));
        ps.sort_by(|a, b| a.x.total_cmp(&b.x).then(a.y.total_cmp(&b.y)));
        println!("{:?}", ps);
        let mut vs = vec![
            Ver { major: 1, minor: 2, tag: None },
            Ver { major: 1, minor: 2, tag: Some("beta".into()) },
            Ver { major: 0, minor: 9, tag: Some("a".into()) },
            Ver::default(),
        ];
        vs.sort();
        for v in &vs { println!("{:?}", v); }
        println!("{:?}", vs.iter().max_by_key(|v| (v.minor, v.tag.clone())));
        println!("{}", P::default() == P { x: 0.0, y: -0.0 });
        let fs = [2.5f32, -1.0, 7.25];
        let mx = fs.iter().cloned().fold(f32::MIN, f32::max);
        println!("{}", mx);
        let mut fv = fs.to_vec();
        fv.sort_by(|a, b| b.partial_cmp(a).unwrap());
        println!("{:?}", fv);
    }
}

mod q07 {
    macro_rules! hashmapish {
        ($($k:expr => $v:expr),* $(,)?) => {{
            let mut v = Vec::new();
            $( v.push(($k, $v)); )*
            v
        }};
    }
    macro_rules! count {
        () => { 0usize };
        ($head:tt $($tail:tt)*) => { 1usize + count!($($tail)*) };
    }
    macro_rules! make_struct {
        ($name:ident { $($f:ident : $t:ty = $d:expr),* }) => {
            #[derive(Debug)]
            struct $name { $($f: $t),* }
            impl Default for $name { fn default() -> Self { $name { $($f: $d),* } } }
        };
    }
    macro_rules! maxm {
        ($x:expr) => { $x };
        ($x:expr, $($rest:expr),+) => {{ let a = $x; let b = maxm!($($rest),+); if a > b { a } else { b } }};
    }
    macro_rules! ops {
        ($($name:ident => $op:tt);*) => { $( fn $name(a: i32, b: i32) -> i32 { a $op b } )* };
    }
    ops!(addf => +; subf => -; mulf => *);
    make_struct!(Cfg { depth: u8 = 3, name: String = "n".to_string() });
    macro_rules! nested {
        ($( [$($x:expr),*] )*) => { vec![ $( vec![$($x * 10),*] ),* ] };
    }
    macro_rules! tlit { ($l:literal) => { concat!("<", $l, ">") }; }
    macro_rules! with_lifetime { ($lt:lifetime, $t:ty) => { fn first<$lt>(x: &$lt [$t]) -> &$lt $t { &x[0] } }; }
    with_lifetime!('a, u16);
    macro_rules! pat_m { ($p:pat) => { matches!(Some(3), $p) }; }
    macro_rules! blockm { ($b:block) => { { let r = $b; r * 2 } }; }
    macro_rules! vis_fn { ($v:vis fn $n:ident) => { $v fn $n() -> &'static str { stringify!($n) } }; }
    vis_fn!(pub fn hello);
    pub fn main() {
        println!("{:?}", hashmapish!{"a" => 1, "b" => 2,});
        println!("{}", count!(a b c d e));
        println!("{:?}", Cfg::default());
        println!("{}", maxm!(3, 9, 2, 7));
        println!("{} {} {}", addf(2, 3), subf(2, 3), mulf(2, 3));
        println!("{:?}", nested!([1, 2] [3] []));
        println!("{}", tlit!("x"));
        println!("{}", first(&[7u16, 8]));
        println!("{} {}", pat_m!(Some(1..=5)), pat_m!(None));
        println!("{}", blockm!({ 1 + 2 }));
        println!("{} {}", hello(), line!() > 0);
        println!("{}", stringify!(a + b * c));
        println!("{}", concat!(1, 2.5, "x", true, 'c'));
        let v: Vec<i32> = vec![];
        assert!(v.is_empty(), "should be empty {}", 1);
        assert_eq!(1 + 1, 2, "math");
        assert_ne!(1, 2);
        debug_assert!(true);
        println!("{}", module_path!());
        println!("{}", file!().ends_with(".rs"));
    }
}

mod q08 {
    use std::fmt;
    #[derive(Debug)]
    enum AppError { Io(String), Parse(std::num::ParseIntError), Neg(i64) }
    impl fmt::Display for AppError {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            match self { AppError::Io(s) => write!(f, "io: {}", s), AppError::Parse(e) => write!(f, "parse: {}", e), AppError::Neg(n) => write!(f, "negative: {}", n) }
        }
    }
    impl std::error::Error for AppError {
        fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
            match self { AppError::Parse(e) => Some(e), _ => None }
        }
    }
    impl From<std::num::ParseIntError> for AppError { fn from(e: std::num::ParseIntError) -> Self { AppError::Parse(e) } }
    fn parse_pos(s: &str) -> Result<i64, AppError> {
        let n: i64 = s.trim().parse()?;
        if n < 0 { return Err(AppError::Neg(n)); }
        Ok(n)
    }
    fn run(s: &str) -> Result<i64, Box<dyn std::error::Error>> {
        let n = parse_pos(s)?;
        if n == 0 { return Err("zero not allowed".into()); }
        if n == 1 { Err(format!("one: {}", n))?; }
        let f: f64 = "2.5".parse()?;
        Ok(n * f as i64)
    }
    fn opt_chain(v: &[i32]) -> Option<i32> {
        let a = v.first()?;
        let b = v.get(1)?;
        Some(a.checked_mul(*b)? + 1)
    }
    pub fn main() {
        for s in ["5", "-3", "abc", "0", "1", " 7 "] {
            match run(s) {
                Ok(v) => println!("ok {}", v),
                Err(e) => {
                    let src = e.source().map(|s| s.to_string());
                    println!("err {} | {:?} | src={:?}", e, e.to_string(), src);
                }
            }
        }
        println!("{:?}", parse_pos("x"));
        println!("{:?} {:?} {:?}", opt_chain(&[2, 3]), opt_chain(&[1]), opt_chain(&[i32::MAX, 2]));
        let e: Box<dyn std::error::Error + Send + Sync> = From::from("plain");
        println!("{} {:?}", e, e);
        let r: Result<(), Box<dyn std::error::Error>> = Err(Box::new(AppError::Io("disk".into())));
        if let Err(e) = r {
            println!("{}", e);
            if let Some(app) = e.downcast_ref::<AppError>() { println!("downcast ok {:?}", app); }
        }
    }
}

fn main() {
    println!("== q01");
    q01::main();
    println!("== q02");
    q02::main();
    println!("== q03");
    q03::main();
    println!("== q04");
    q04::main();
    println!("== q05");
    q05::main();
    println!("== q06");
    q06::main();
    println!("== q07");
    q07::main();
    println!("== q08");
    q08::main();
}
