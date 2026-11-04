use std::cell::RefCell;
use std::collections::HashMap;
use std::hash::Hash;
use std::rc::Rc;

fn fix<A, R>(f: &dyn Fn(&dyn Fn(A) -> R, A) -> R, a: A) -> R {
    f(&|x| fix(f, x), a)
}

fn memo_fix<A, R>(f: impl Fn(&dyn Fn(A) -> R, A) -> R) -> impl Fn(A) -> R
where A: Hash + Eq + Clone, R: Clone {
    let cache: Rc<RefCell<HashMap<A, R>>> = Rc::new(RefCell::new(HashMap::new()));
    let f = Rc::new(f);
    fn go<A: Hash + Eq + Clone, R: Clone, F: Fn(&dyn Fn(A) -> R, A) -> R>(f: &Rc<F>, c: &Rc<RefCell<HashMap<A, R>>>, a: A) -> R {
        if let Some(r) = c.borrow().get(&a) { return r.clone(); }
        let r = f(&|x| go(f, c, x), a.clone());
        c.borrow_mut().insert(a, r.clone());
        r
    }
    move |a| go(&f, &cache, a)
}

struct Pipeline<T> { stages: Vec<(String, Box<dyn Fn(T) -> Option<T>>)> }
impl<T: Clone + std::fmt::Debug + 'static> Pipeline<T> {
    fn new() -> Self { Pipeline { stages: vec![] } }
    fn stage(mut self, name: &str, f: impl Fn(T) -> Option<T> + 'static) -> Self { self.stages.push((name.to_string(), Box::new(f))); self }
    fn map(self, name: &str, f: impl Fn(T) -> T + 'static) -> Self { self.stage(name, move |x| Some(f(x))) }
    fn filter(self, name: &str, p: impl Fn(&T) -> bool + 'static) -> Self { self.stage(name, move |x| if p(&x) { Some(x) } else { None }) }
    fn run(&self, x: T) -> Result<T, String> {
        let mut cur = x;
        for (n, s) in &self.stages {
            cur = s(cur.clone()).ok_or_else(|| format!("rejected at {} with {:?}", n, cur))?;
        }
        Ok(cur)
    }
}

trait Observer<E> { fn notify(&self, e: &E); }
impl<E, F: Fn(&E)> Observer<E> for F { fn notify(&self, e: &E) { self(e) } }
struct Bus<'a, E> { subs: Vec<Box<dyn Observer<E> + 'a>> }
impl<'a, E> Bus<'a, E> {
    fn subscribe(&mut self, o: impl Observer<E> + 'a) { self.subs.push(Box::new(o)); }
    fn emit(&self, e: E) { for s in &self.subs { s.notify(&e); } }
}

fn main() {
    let fact = |rec: &dyn Fn(u64) -> u64, n: u64| if n == 0 { 1 } else { n * rec(n - 1) };
    println!("{}", fix(&fact, 10));
    let fib = memo_fix(|rec: &dyn Fn(u64) -> u64, n: u64| if n < 2 { n } else { rec(n - 1) + rec(n - 2) });
    println!("{} {}", fib(80), fib(10));
    let paths = memo_fix(|rec: &dyn Fn((u32, u32)) -> u64, (r, c): (u32, u32)| if r == 0 || c == 0 { 1 } else { rec((r - 1, c)) + rec((r, c - 1)) });
    println!("{}", paths((16, 16)));
    let p = Pipeline::new().map("double", |x: i64| x * 2).filter("small", |x| *x < 100).map("inc", |x| x + 1);
    println!("{:?} {:?}", p.run(10), p.run(60));
    let ps = Pipeline::new().map("trim", |s: String| s.trim().to_string()).filter("nonempty", |s| !s.is_empty());
    println!("{:?} {:?}", ps.run("  hi ".into()), ps.run("   ".into()));
    let log = RefCell::new(Vec::new());
    let count = std::cell::Cell::new(0);
    {
        let mut bus: Bus<(&str, i32)> = Bus { subs: vec![] };
        bus.subscribe(|e: &(&str, i32)| log.borrow_mut().push(format!("{}={}", e.0, e.1)));
        bus.subscribe(|_: &(&str, i32)| count.set(count.get() + 1));
        bus.emit(("a", 1));
        bus.emit(("b", 2));
    }
    println!("{:?} {}", log.borrow(), count.get());
}
