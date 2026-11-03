// FnOnce / FnMut / Fn passed through several generic layers, by value and by &mut
fn call_once<F: FnOnce() -> String>(f: F) -> String {
    f()
}
fn layer1<F: FnOnce() -> String>(f: F) -> String {
    layer2(f)
}
fn layer2<F>(f: F) -> String
where
    F: FnOnce() -> String,
{
    format!("<{}>", call_once(f))
}

fn call_mut_n<F: FnMut(usize)>(mut f: F, n: usize) {
    for i in 0..n {
        f(i);
    }
}
fn via_ref<F: FnMut(usize)>(f: &mut F) {
    f(100);
    f(200);
}
fn through_dyn(f: &mut dyn FnMut(usize)) {
    f(7);
}
fn pass_along<F: FnMut(usize)>(mut f: F) -> F {
    via_ref(&mut f);
    through_dyn(&mut f);
    f
}

fn maybe_call<F: FnOnce(i32) -> i32>(f: Option<F>, v: i32) -> i32 {
    match f {
        Some(f) => f(v),
        None => v,
    }
}

struct Deferred<F: FnOnce() -> Vec<String>> {
    f: Option<F>,
}
impl<F: FnOnce() -> Vec<String>> Deferred<F> {
    fn run(&mut self) -> Vec<String> {
        match self.f.take() {
            Some(f) => f(),
            None => vec!["already ran".to_string()],
        }
    }
}

fn boxed_once(v: Vec<i32>) -> Box<dyn FnOnce() -> i32> {
    Box::new(move || v.into_iter().sum())
}

fn generic_fn<F>(f: F) -> impl Fn(i32) -> i32
where
    F: Fn(i32) -> i32 + Clone,
{
    let g = f.clone();
    move |x| f(g(x))
}

fn run_all(tasks: Vec<Box<dyn FnOnce(&mut Vec<String>)>>) -> Vec<String> {
    let mut log = vec![];
    for t in tasks {
        t(&mut log);
    }
    log
}

fn main() {
    let s = String::from("owned");
    println!("{}", layer1(move || s + "!"));
    let mut hits = vec![];
    call_mut_n(|i| hits.push(i * i), 4);
    println!("{:?}", hits);
    let mut total = 0;
    let f = pass_along(|x| total += x);
    drop(f);
    println!("total={}", total);
    let mut log = String::new();
    {
        let mut appender = |x: usize| log.push_str(&format!("{};", x));
        call_mut_n(&mut appender, 3);
        via_ref(&mut appender);
        through_dyn(&mut appender);
    }
    println!("{}", log);
    let offset = 5;
    println!("{}", maybe_call(Some(|x| x + offset), 10));
    println!("{}", maybe_call(None::<fn(i32) -> i32>, 10));
    let name = String::from("job");
    let mut d = Deferred {
        f: Some(move || vec![name.clone(), name.to_uppercase()]),
    };
    println!("{:?}", d.run());
    println!("{:?}", d.run());
    let b = boxed_once(vec![1, 2, 3, 4]);
    println!("{}", b());
    let sq = generic_fn(|x| x * x);
    println!("{}", sq(3));
    let tasks: Vec<Box<dyn FnOnce(&mut Vec<String>)>> = vec![
        Box::new(|l| l.push("first".into())),
        Box::new({
            let owned = vec![1, 2];
            move |l| l.push(format!("{:?}", owned))
        }),
        Box::new(|l| {
            let n = l.len();
            l.push(format!("n={}", n))
        }),
    ];
    println!("{:?}", run_all(tasks));
    let mut counter = 0;
    let mut inc = || {
        counter += 3;
        counter
    };
    let a = inc();
    let b2 = inc();
    println!("{} {} {}", a, b2, counter);
    let v = vec![String::from("x"), String::from("y")];
    let consume = move || v.len();
    println!("{} {}", consume(), consume());
}
