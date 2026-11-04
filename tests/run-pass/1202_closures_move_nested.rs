fn make_adder(n: i32) -> impl Fn(i32) -> i32 {
    move |x| x + n
}

fn make_counter() -> impl FnMut() -> u32 {
    let mut c = 0;
    move || {
        c += 1;
        c
    }
}

fn compose<A, B, C>(f: impl Fn(A) -> B, g: impl Fn(B) -> C) -> impl Fn(A) -> C {
    move |x| g(f(x))
}

fn curry(a: i32) -> impl Fn(i32) -> Box<dyn Fn(i32) -> i32> {
    move |b| Box::new(move |c| a * 100 + b * 10 + c)
}

fn apply_n(f: &dyn Fn(i64) -> i64, n: usize, x: i64) -> i64 {
    (0..n).fold(x, |acc, _| f(acc))
}

fn make_ops() -> Vec<(&'static str, Box<dyn Fn(i32, i32) -> i32>)> {
    let offset = 1000;
    vec![
        ("add", Box::new(|a, b| a + b)),
        ("mul", Box::new(|a, b| a * b)),
        ("off", Box::new(move |a, b| a - b + offset)),
    ]
}

fn once_consumer<F: FnOnce() -> String>(f: F) -> String {
    f()
}

struct Logger {
    lines: Vec<String>,
}

impl Logger {
    fn with<R>(&mut self, tag: &str, f: impl FnOnce(&mut Vec<String>) -> R) -> R {
        self.lines.push(format!("begin {}", tag));
        let r = f(&mut self.lines);
        self.lines.push(format!("end {}", tag));
        r
    }
}

fn main() {
    let add5 = make_adder(5);
    println!("{}", add5(10));
    let mut c1 = make_counter();
    let mut c2 = make_counter();
    println!("{} {} {} {}", c1(), c1(), c2(), c1());

    let f = compose(|x: i32| x * 2, |y: i32| format!("<{}>", y));
    println!("{}", f(21));
    let g = compose(make_adder(1), make_adder(2));
    println!("{}", g(0));

    let h = curry(1);
    let h2 = h(2);
    println!("{} {}", h2(3), h(4)(5));

    let k = 3;
    println!("{}", apply_n(&|x| x * k, 4, 1));

    for (name, op) in make_ops().iter() {
        println!("{} -> {}", name, op(7, 3));
    }

    let s = String::from("owned");
    let msg = once_consumer(move || s + "!");
    println!("{}", msg);

    // nested move closures capturing outer captured vars
    let base = vec![1, 2, 3];
    let outer = move |m: i32| {
        let base2 = base.clone();
        move |i: usize| base2[i] * m
    };
    let inner = outer(10);
    println!("{} {}", inner(0), inner(2));

    // closure capturing by unique borrow, then used after
    let mut v = vec![1];
    {
        let mut push = |x| v.push(x);
        push(2);
        push(3);
    }
    println!("{:?}", v);

    let mut lg = Logger { lines: vec![] };
    let n = lg.with("a", |l| {
        l.push("inside".to_string());
        l.len()
    });
    println!("{} {:?}", n, lg.lines);

    // closures stored in a struct field and called
    struct Btn<F: Fn() -> String> {
        on_click: F,
    }
    let label = "ok".to_string();
    let b = Btn { on_click: || format!("clicked {}", label) };
    println!("{}", (b.on_click)());
    println!("{}", label);

    // returning closures from match arms via Box
    let fs: Vec<Box<dyn Fn(f64) -> f64>> = (0..3)
        .map(|i| -> Box<dyn Fn(f64) -> f64> {
            match i {
                0 => Box::new(|x| x + 0.5),
                1 => Box::new(move |x| x * i as f64 * 3.0),
                _ => Box::new(move |x| x.powi(i)),
            }
        })
        .collect();
    let rs: Vec<f64> = fs.iter().map(|f| f(2.0)).collect();
    println!("{:?}", rs);

    // recursion via fn pointer + closure
    fn fix(f: &dyn Fn(&dyn Fn(u64) -> u64, u64) -> u64, n: u64) -> u64 {
        f(&|k| fix(f, k), n)
    }
    let fact = fix(&|rec, n| if n == 0 { 1 } else { n * rec(n - 1) }, 10);
    println!("{}", fact);
}
