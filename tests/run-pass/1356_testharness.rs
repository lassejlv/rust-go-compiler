// A mini property-testing + unit-test harness: test registry of fn pointers
// and boxed closures, panics caught with catch_unwind (custom hook silences
// output), shrinking of failing inputs, drop-guards observed during unwinding,
// panics in threads, and checked/wrapping/saturating arithmetic semantics.
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::fmt::Debug;
use std::panic::{self, AssertUnwindSafe};
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use std::thread;

thread_local! {
    static DROPS: RefCell<Vec<String>> = RefCell::new(Vec::new());
}

struct Guard(&'static str);
impl Drop for Guard {
    fn drop(&mut self) {
        DROPS.with(|d| d.borrow_mut().push(self.0.to_string()));
    }
}

#[derive(Debug, Clone, PartialEq)]
enum Outcome {
    Pass,
    Fail(String),
    Panic(String),
}

fn panic_message(e: &(dyn std::any::Any + Send)) -> String {
    if let Some(s) = e.downcast_ref::<&str>() {
        s.to_string()
    } else if let Some(s) = e.downcast_ref::<String>() {
        s.clone()
    } else {
        "<non-string panic>".to_string()
    }
}

type TestFn = Box<dyn Fn() -> Result<(), String>>;

struct Suite {
    tests: Vec<(&'static str, TestFn)>,
}

impl Suite {
    fn add(&mut self, name: &'static str, f: impl Fn() -> Result<(), String> + 'static) -> &mut Self {
        self.tests.push((name, Box::new(f)));
        self
    }
    fn add_fn(&mut self, name: &'static str, f: fn()) -> &mut Self {
        self.add(name, move || {
            f();
            Ok(())
        })
    }
    fn run(&self) -> BTreeMap<&'static str, Outcome> {
        let mut out = BTreeMap::new();
        for (name, t) in &self.tests {
            let r = panic::catch_unwind(AssertUnwindSafe(|| t()));
            let o = match r {
                Ok(Ok(())) => Outcome::Pass,
                Ok(Err(m)) => Outcome::Fail(m),
                Err(e) => Outcome::Panic(panic_message(&*e)),
            };
            out.insert(*name, o);
        }
        out
    }
}

macro_rules! check_eq {
    ($a:expr, $b:expr) => {{
        let (a, b) = (&$a, &$b);
        if a != b {
            return Err(format!("{} != {} ({:?} vs {:?})", stringify!($a), stringify!($b), a, b));
        }
    }};
}

// ---------- property testing ----------
struct Gen {
    state: u64,
}

impl Gen {
    fn u64(&mut self) -> u64 {
        // splitmix64
        self.state = self.state.wrapping_add(0x9E3779B97F4A7C15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
        z ^ (z >> 31)
    }
    fn range(&mut self, lo: i64, hi: i64) -> i64 {
        lo + (self.u64() % (hi - lo) as u64) as i64
    }
    fn vec(&mut self, max_len: usize) -> Vec<i64> {
        let n = self.u64() as usize % (max_len + 1);
        (0..n).map(|_| self.range(-50, 50)).collect()
    }
}

trait Shrink: Sized {
    fn shrink(&self) -> Vec<Self>;
}

impl Shrink for Vec<i64> {
    fn shrink(&self) -> Vec<Vec<i64>> {
        let mut out = Vec::new();
        if self.is_empty() {
            return out;
        }
        out.push(self[..self.len() / 2].to_vec());
        out.push(self[self.len() / 2..].to_vec());
        for i in 0..self.len() {
            let mut v = self.clone();
            v.remove(i);
            out.push(v);
        }
        for i in 0..self.len() {
            if self[i] != 0 {
                let mut v = self.clone();
                v[i] /= 2;
                out.push(v);
            }
        }
        out
    }
}

fn forall<T: Shrink + Debug + Clone>(cases: usize, mut gen: impl FnMut(&mut Gen) -> T, prop: impl Fn(&T) -> bool) -> Result<usize, (T, usize)> {
    let mut g = Gen { state: 7 };
    for i in 0..cases {
        let x = gen(&mut g);
        let holds = panic::catch_unwind(AssertUnwindSafe(|| prop(&x))).unwrap_or(false);
        if !holds {
            // shrink greedily
            let mut cur = x;
            let mut steps = 0;
            'outer: loop {
                for cand in cur.shrink() {
                    let ok = panic::catch_unwind(AssertUnwindSafe(|| prop(&cand))).unwrap_or(false);
                    if !ok {
                        cur = cand;
                        steps += 1;
                        continue 'outer;
                    }
                }
                break;
            }
            let _ = i;
            return Err((cur, steps));
        }
    }
    Ok(cases)
}

fn buggy_sort(v: &mut Vec<i64>) {
    // bubble sort that forgets the last pass
    let n = v.len();
    for i in 0..n.saturating_sub(2) {
        for j in 0..n - 1 - i {
            if v[j] > v[j + 1] {
                v.swap(j, j + 1);
            }
        }
    }
}

fn mean(v: &[i64]) -> i64 {
    v.iter().sum::<i64>() / v.len() as i64
}

fn deep_unwind(n: u32) -> u32 {
    let _g = Guard(if n % 2 == 0 { "even-frame" } else { "odd-frame" });
    if n == 0 {
        let v: Vec<u32> = Vec::new();
        return v[3];
    }
    deep_unwind(n - 1) + 1
}

fn main() {
    panic::set_hook(Box::new(|_| {}));
    let shared = Rc::new(RefCell::new(0));
    let s2 = shared.clone();
    let mut suite = Suite { tests: vec![] };
    suite
        .add("arith", || {
            check_eq!(2 + 2, 4);
            check_eq!(7 / 2, 3);
            check_eq!(-7 % 3, -1);
            check_eq!((-7i32).rem_euclid(3), 2);
            check_eq!((-7i32).div_euclid(3), -3);
            Ok(())
        })
        .add("strings", || {
            check_eq!("abc".repeat(2), "abcabc");
            check_eq!("a,b,,c".split(',').collect::<Vec<_>>(), vec!["a", "b", "", "c"]);
            check_eq!("Hello".to_uppercase(), "HELLO!");
            Ok(())
        })
        .add("closure-state", move || {
            *s2.borrow_mut() += 1;
            check_eq!(*s2.borrow(), 1);
            Ok(())
        })
        .add_fn("index-oob", || {
            let v = vec![1, 2, 3];
            let i = v.len() + 2;
            println!("{}", v[i]);
        })
        .add_fn("unwrap-none", || {
            let m: BTreeMap<i32, i32> = BTreeMap::new();
            let _ = m.get(&1).copied().unwrap();
        })
        .add_fn("expect-err", || {
            let r: Result<u8, String> = Err("boom".into());
            r.expect("config should load");
        })
        .add_fn("explicit-panic", || panic!("custom {} {}", 42, "message"))
        .add_fn("overflow", || {
            let x: u8 = "200".parse().unwrap();
            let y = x.checked_add(100);
            assert!(y.is_none());
            let z = std::hint::black_box(x).wrapping_add(100);
            assert_eq!(z, 44, "wrapped");
        })
        .add_fn("div-zero", || {
            let d = std::hint::black_box(0);
            println!("{}", 10 / d);
        })
        .add_fn("assert-eq", || assert_eq!(vec![1, 2], vec![1, 3], "vectors differ"))
        .add_fn("assert-ne", || assert_ne!(1 + 1, 2))
        .add_fn("refcell-double-borrow", || {
            let c = RefCell::new(5);
            let _a = c.borrow_mut();
            let _b = c.borrow_mut();
        })
        .add_fn("slice-range", || {
            let s = "hello";
            let n = std::hint::black_box(10);
            println!("{}", &s[2..n]);
        })
        .add_fn("char-boundary", || {
            let s = "héllo";
            let n = std::hint::black_box(2);
            println!("{}", &s[..n]);
        })
        .add_fn("unreachable", || unreachable!("state {}", 3))
        .add_fn("todo", || todo!());
    let results = suite.run();
    for (name, o) in &results {
        println!("{:<22} {:?}", name, o);
    }
    let pass = results.values().filter(|o| **o == Outcome::Pass).count();
    println!("{} passed, {} failed, shared counter {}", pass, results.len() - pass, shared.borrow());

    // drop guards during unwinding
    let r = panic::catch_unwind(|| deep_unwind(4));
    DROPS.with(|d| println!("unwind: err={} drops={:?}", r.is_err(), d.borrow()));

    // property tests
    let sort_prop = |v: &Vec<i64>| {
        let mut a = v.clone();
        buggy_sort(&mut a);
        a.windows(2).all(|w| w[0] <= w[1])
    };
    match forall(200, |g| g.vec(8), sort_prop) {
        Ok(n) => println!("sort: {} cases ok", n),
        Err((cex, steps)) => println!("sort: counterexample {:?} after {} shrinks", cex, steps),
    }
    let rev_prop = |v: &Vec<i64>| {
        let mut a = v.clone();
        a.reverse();
        a.reverse();
        a == *v
    };
    println!("reverse: {:?}", forall(200, |g| g.vec(10), rev_prop).map_err(|e| e.0));
    // mean panics on empty input -> caught as failure
    match forall(50, |g| g.vec(3), |v: &Vec<i64>| mean(v).abs() <= 50) {
        Ok(n) => println!("mean: ok {}", n),
        Err((cex, _)) => println!("mean: counterexample {:?}", cex),
    }
    let sum_prop = |v: &Vec<i64>| v.iter().sum::<i64>() < 100;
    println!("sum<100: {:?}", forall(300, |g| g.vec(12), sum_prop).map_err(|(c, s)| (c.len(), c.iter().sum::<i64>(), s > 0)));

    // threads that panic
    let log = Arc::new(Mutex::new(Vec::new()));
    let handles: Vec<_> = (0..4)
        .map(|i| {
            let log = Arc::clone(&log);
            thread::spawn(move || {
                if i == 2 {
                    panic!("worker {} failed", i);
                }
                log.lock().unwrap().push(i);
                i * 10
            })
        })
        .collect();
    let joined: Vec<Result<i32, String>> = handles.into_iter().map(|h| h.join().map_err(|e| panic_message(&*e))).collect();
    let mut l = log.lock().unwrap().clone();
    l.sort();
    println!("threads: {:?} log {:?}", joined, l);
    // poisoned mutex
    let m = Arc::new(Mutex::new(1));
    let m2 = m.clone();
    let _ = thread::spawn(move || {
        let _g = m2.lock().unwrap();
        panic!("poison");
    })
    .join();
    println!("poisoned: {} value {:?}", m.is_poisoned(), m.lock().map(|g| *g).unwrap_or_else(|e| *e.into_inner()));

    // integer semantics
    let x: i32 = i32::MAX;
    println!("{:?} {} {} {:?} {:?}", x.checked_mul(2), x.wrapping_add(1), x.saturating_add(5), x.overflowing_sub(-1), 5u32.checked_sub(6));
    println!("{} {} {} {}", (-128i8).wrapping_neg(), (-128i8).saturating_neg(), i64::MIN.wrapping_abs(), (-5i32).unsigned_abs());
    println!("{} {} {} {}", 1u64.rotate_right(1), 0x1234u16.swap_bytes(), 200u8.leading_zeros(), (-1i16).count_ones());
    println!("{:?} {:?} {:?}", 10u32.checked_pow(10), 3i64.checked_pow(39), 2u128.pow(100));
    println!("{} {} {}", u64::MAX.wrapping_mul(3), (1u32 << 31).wrapping_shl(1), 7i32.pow(2).isqrt());
    let r = panic::catch_unwind(|| {
        let a = std::hint::black_box(i32::MIN);
        a / -1
    });
    println!("MIN / -1 panics: {}", r.is_err());
    println!("checked shift: {:?} {:?}", 1u32.checked_shl(40), 1u32.checked_shl(4));
    let _ = panic::take_hook();
}
