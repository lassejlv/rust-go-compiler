// Closures stored in generic struct fields (F: Fn), with methods that call them,
// an event emitter with Vec<Box<dyn FnMut>>, lazy values, and a validator chain.
use std::fmt::Debug;

struct Mapper<F, T>
where
    F: Fn(T) -> T,
{
    f: F,
    times: usize,
    _t: std::marker::PhantomData<T>,
}

impl<F: Fn(T) -> T, T: Copy + Debug> Mapper<F, T> {
    fn new(f: F, times: usize) -> Self {
        Mapper { f, times, _t: std::marker::PhantomData }
    }
    fn run(&self, start: T) -> Vec<T> {
        let mut out = vec![start];
        let mut cur = start;
        for _ in 0..self.times {
            cur = (self.f)(cur);
            out.push(cur);
        }
        out
    }
}

struct Lazy<T, F: FnOnce() -> T> {
    init: Option<F>,
    value: Option<T>,
}
impl<T: Debug, F: FnOnce() -> T> Lazy<T, F> {
    fn new(f: F) -> Self {
        Lazy { init: Some(f), value: None }
    }
    fn get(&mut self) -> &T {
        if self.value.is_none() {
            let f = self.init.take().expect("init");
            println!("  (computing)");
            self.value = Some(f());
        }
        self.value.as_ref().unwrap()
    }
}

struct Emitter<'a> {
    listeners: Vec<(String, Box<dyn FnMut(&str) -> Option<String> + 'a>)>,
}
impl<'a> Emitter<'a> {
    fn on(&mut self, name: &str, f: impl FnMut(&str) -> Option<String> + 'a) {
        self.listeners.push((name.to_string(), Box::new(f)));
    }
    fn emit(&mut self, payload: &str) -> Vec<String> {
        let mut out = Vec::new();
        for (name, l) in self.listeners.iter_mut() {
            if let Some(r) = l(payload) {
                out.push(format!("{}->{}", name, r));
            }
        }
        out
    }
}

struct Validator<T> {
    rules: Vec<(&'static str, Box<dyn Fn(&T) -> bool>)>,
}
impl<T> Validator<T> {
    fn new() -> Self {
        Validator { rules: Vec::new() }
    }
    fn rule(mut self, name: &'static str, f: impl Fn(&T) -> bool + 'static) -> Self {
        self.rules.push((name, Box::new(f)));
        self
    }
    fn check(&self, v: &T) -> Result<(), Vec<&'static str>> {
        let failed: Vec<&'static str> = self.rules.iter().filter(|(_, f)| !f(v)).map(|(n, _)| *n).collect();
        if failed.is_empty() {
            Ok(())
        } else {
            Err(failed)
        }
    }
}

fn main() {
    let m = Mapper::new(|x: i32| x * 2 + 1, 5);
    println!("{:?}", m.run(0));
    let m2 = Mapper::new(|x: f64| x / 2.0, 3);
    println!("{:?}", m2.run(10.0));

    let base = 7;
    let mut lazy = Lazy::new(move || (1..=base).product::<u64>());
    println!("first get");
    println!("value {}", lazy.get());
    println!("second get");
    println!("value {}", lazy.get());

    let mut seen: Vec<String> = Vec::new();
    {
        let mut count = 0;
        let mut em = Emitter { listeners: Vec::new() };
        em.on("len", |p| Some(p.len().to_string()));
        em.on("count", move |_| {
            count += 1;
            Some(format!("#{}", count))
        });
        em.on("filter", |p| if p.starts_with('a') { Some(p.to_uppercase()) } else { None });
        em.on("record", |p| {
            seen.push(p.to_string());
            None
        });
        for p in ["apple", "kiwi", "avocado"] {
            println!("{:?}", em.emit(p));
        }
    }
    println!("recorded {:?}", seen);

    let max_len = 8;
    let v = Validator::<String>::new()
        .rule("nonempty", |s| !s.is_empty())
        .rule("short", move |s| s.len() <= max_len)
        .rule("lower", |s| s.chars().all(|c| !c.is_uppercase()))
        .rule("no-space", |s| !s.contains(' '));
    for s in ["ok", "", "Too Long String", "fine_one"] {
        match v.check(&s.to_string()) {
            Ok(()) => println!("{:?} valid", s),
            Err(e) => println!("{:?} failed {:?}", s, e),
        }
    }
}
