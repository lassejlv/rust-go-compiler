fn make_adder(n: i32) -> impl Fn(i32) -> i32 {
    move |x| x + n
}

fn make_multiplier(n: i32) -> Box<dyn Fn(i32) -> i32> {
    Box::new(move |x| x * n)
}

fn make_op(name: &str) -> Box<dyn Fn(i32, i32) -> i32> {
    match name {
        "add" => Box::new(|a, b| a + b),
        "pow" => Box::new(|a, b| a.pow(b as u32)),
        _ => Box::new(|_, _| 0),
    }
}

fn compose<A, B, C>(f: impl Fn(A) -> B, g: impl Fn(B) -> C) -> impl Fn(A) -> C {
    move |x| g(f(x))
}

fn make_counter() -> impl FnMut() -> u32 {
    let mut count = 0;
    move || {
        count += 1;
        count
    }
}

fn make_accumulator(start: f64) -> Box<dyn FnMut(f64) -> f64> {
    let mut total = start;
    Box::new(move |x| {
        total += x;
        total
    })
}

fn main() {
    let add5 = make_adder(5);
    let times3 = make_multiplier(3);
    println!("add5(10) = {}", add5(10));
    println!("times3(10) = {}", times3(10));
    println!("add pow other = {} {} {}", make_op("add")(2, 3), make_op("pow")(2, 10), make_op("?")(1, 1));
    let both = compose(add5, times3);
    println!("compose = {}", both(1));
    let to_str = compose(|x: i32| x * x, |y: i32| format!("<{}>", y));
    println!("compose types = {}", to_str(9));
    let mut c1 = make_counter();
    let mut c2 = make_counter();
    println!("counters: {} {} {} | {}", c1(), c1(), c1(), c2());
    let mut acc = make_accumulator(100.0);
    acc(10.0);
    acc(-25.5);
    println!("accumulator = {}", acc(0.5));
    let pipeline: Vec<Box<dyn Fn(i32) -> i32>> = vec![
        Box::new(|x| x + 1),
        Box::new(|x| x * 10),
        make_multiplier(2),
        Box::new(make_adder(-7)),
    ];
    let result = pipeline.iter().fold(4, |acc, f| f(acc));
    println!("pipeline(4) = {}", result);
    let adders: Vec<Box<dyn Fn(i32) -> i32>> = (1..=3).map(|n| Box::new(move |x| x + n) as Box<dyn Fn(i32) -> i32>).collect();
    let outs: Vec<i32> = adders.iter().map(|f| f(100)).collect();
    println!("adders = {:?}", outs);
}
