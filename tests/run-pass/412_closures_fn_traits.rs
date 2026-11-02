// FnOnce / FnMut / Fn passed through generics, &dyn, &mut dyn and Box<dyn>,
// closures coerced to fn pointers, and calling closures stored in Options.
fn apply_once<F: FnOnce() -> String>(f: F) -> String {
    f()
}
fn apply_mut<F: FnMut(i32)>(mut f: F, n: i32) {
    for i in 0..n {
        f(i);
    }
}
fn apply_fn<F>(f: &F, x: i32) -> i32
where
    F: Fn(i32) -> i32 + ?Sized,
{
    f(f(x))
}
fn dyn_mut(f: &mut dyn FnMut() -> u32) -> u32 {
    f() + f() + f()
}
fn boxed_once(f: Box<dyn FnOnce(String) -> usize>) -> usize {
    f("abcdef".to_string())
}
fn fn_ptr(f: fn(u8) -> u8, v: u8) -> u8 {
    f(v)
}
fn compose<A, B, C>(f: impl Fn(A) -> B, g: impl Fn(B) -> C) -> impl Fn(A) -> C {
    move |x| g(f(x))
}
fn double(x: u8) -> u8 {
    x.wrapping_mul(2)
}

struct Button {
    label: String,
    on_click: Option<Box<dyn FnMut(&str) -> String>>,
}

impl Button {
    fn click(&mut self) -> String {
        match self.on_click.as_mut() {
            Some(cb) => cb(&self.label),
            None => format!("{} has no handler", self.label),
        }
    }
}

fn main() {
    let name = String::from("world");
    let greeting = apply_once(move || format!("hello {}", name));
    println!("{}", greeting);

    let mut acc = Vec::new();
    apply_mut(|i| acc.push(i * i), 5);
    println!("{:?}", acc);

    let mut sum = 0;
    apply_mut(|i| sum += i, 10);
    println!("sum={}", sum);

    let k = 3;
    let add_k = |x| x + k;
    println!("fn generic: {}", apply_fn(&add_k, 10));
    let as_dyn: &dyn Fn(i32) -> i32 = &add_k;
    println!("fn dyn: {}", apply_fn(as_dyn, 1));

    let mut counter = 0u32;
    let mut tick = || {
        counter += 1;
        counter * 10
    };
    println!("dyn mut: {}", dyn_mut(&mut tick));
    println!("counter after: {}", counter);

    let suffix = String::from("!!");
    println!("boxed once: {}", boxed_once(Box::new(move |s| s.len() + suffix.len())));

    println!("fn ptr from fn: {}", fn_ptr(double, 200));
    println!("fn ptr from closure: {}", fn_ptr(|v| v ^ 0xff, 15));
    let table: [(&str, fn(u8) -> u8); 3] = [("dbl", double), ("inc", |v| v + 1), ("neg", |v| v.wrapping_neg())];
    for (n, f) in table.iter() {
        print!("{}={} ", n, f(7));
    }
    println!();

    let f = compose(|x: i32| x * 2, |y: i32| format!("<{}>", y));
    let g = compose(&f, |s: String| s.len());
    println!("{} {}", f(21), g(12345));

    let mut log = Vec::new();
    let mut b1 = Button {
        label: "ok".into(),
        on_click: Some(Box::new(|l| format!("clicked {}", l))),
    };
    let mut clicks = 0;
    let mut b2 = Button {
        label: "count".into(),
        on_click: Some(Box::new(move |l| {
            clicks += 1;
            format!("{} x{}", l, clicks)
        })),
    };
    let mut b3 = Button { label: "none".into(), on_click: None };
    for _ in 0..2 {
        log.push(b1.click());
        log.push(b2.click());
        log.push(b3.click());
    }
    println!("{}", log.join("; "));

    let fs: Vec<Box<dyn Fn(i32) -> i32>> = (1..=3).map(|m| Box::new(move |x| x * m) as Box<dyn Fn(i32) -> i32>).collect();
    let results: Vec<i32> = fs.iter().map(|f| f(10)).collect();
    println!("{:?}", results);
    let folded = fs.iter().fold(1, |acc, f| f(acc));
    println!("folded {}", folded);
}
