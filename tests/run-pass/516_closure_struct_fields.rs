// closures stored in struct fields, called via (self.f)(), generic and boxed
struct Button<F: Fn(&str) -> String> {
    label: String,
    on_click: F,
}
impl<F: Fn(&str) -> String> Button<F> {
    fn click(&self) -> String {
        (self.on_click)(&self.label)
    }
}

struct Machine {
    state: i32,
    step: Box<dyn FnMut(i32) -> i32>,
    check: fn(i32) -> bool,
}
impl Machine {
    fn run(&mut self, max: usize) -> Vec<i32> {
        let mut out = vec![self.state];
        for _ in 0..max {
            self.state = (self.step)(self.state);
            out.push(self.state);
            if !(self.check)(self.state) {
                break;
            }
        }
        out
    }
}

struct Lazy<T, F: FnOnce() -> T> {
    init: Option<F>,
    value: Option<T>,
}
impl<T: Clone, F: FnOnce() -> T> Lazy<T, F> {
    fn new(f: F) -> Self {
        Lazy { init: Some(f), value: None }
    }
    fn get(&mut self) -> T {
        if self.value.is_none() {
            let f = self.init.take().unwrap();
            self.value = Some(f());
        }
        self.value.clone().unwrap()
    }
}

struct Validator {
    rules: Vec<(&'static str, Box<dyn Fn(&str) -> bool>)>,
}
impl Validator {
    fn check(&self, s: &str) -> Vec<&'static str> {
        self.rules.iter().filter(|(_, r)| !r(s)).map(|(n, _)| *n).collect()
    }
}

struct Ops {
    a: fn(i32) -> i32,
    b: Box<dyn Fn(i32) -> i32>,
}
impl Ops {
    fn a(&self, x: i32) -> i32 {
        x * 1000
    }
    fn both(&self, x: i32) -> (i32, i32, i32) {
        (self.a(x), (self.a)(x), (self.b)(x))
    }
}

fn main() {
    let prefix = String::from(">> ");
    let b = Button { label: "OK".into(), on_click: |l| format!("{}clicked {}", prefix, l) };
    println!("{}", b.click());
    let b2 = Button { label: "Cancel".into(), on_click: str::to_lowercase };
    println!("{}", b2.click());
    let mut calls = 0;
    let mut m = Machine {
        state: 7,
        step: Box::new(move |s| {
            calls += 1;
            if s % 2 == 0 { s / 2 } else { 3 * s + 1 + calls * 0 }
        }),
        check: |s| s != 1,
    };
    println!("{:?}", m.run(100));
    m.state = 3;
    m.check = |s| s < 10;
    println!("{:?}", m.run(5));
    let mut counter = 0;
    let mut lz = Lazy::new(|| {
        counter += 1;
        vec![1, 2, 3]
    });
    println!("{:?} {:?}", lz.get(), lz.get());
    drop(lz);
    println!("init ran {} time(s)", counter);
    let min_len = 6;
    let v = Validator {
        rules: vec![
            ("too short", Box::new(move |s: &str| s.len() >= min_len)),
            ("no digit", Box::new(|s: &str| s.chars().any(|c| c.is_ascii_digit()))),
            ("no upper", Box::new(|s: &str| s.chars().any(char::is_uppercase))),
            ("has space", Box::new(|s: &str| !s.contains(' '))),
        ],
    };
    for pw in ["abc", "Password1", "pass word", "longenough"] {
        println!("{:?} -> {:?}", pw, v.check(pw));
    }
    let o = Ops { a: |x| x + 1, b: Box::new(|x| x - 1) };
    println!("{:?}", o.both(5));
    let tuple_of_closures = (|x: i32| x * 2, |s: &str| s.len());
    println!("{} {}", (tuple_of_closures.0)(21), (tuple_of_closures.1)("four"));
    let arr_fns: [&dyn Fn(u8) -> u8; 3] = [&|x| x, &|x| x.wrapping_mul(2), &|x| !x];
    println!("{:?}", arr_fns.iter().map(|f| f(200)).collect::<Vec<_>>());
}
