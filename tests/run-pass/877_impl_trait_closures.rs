fn make_adder(n: i32) -> impl Fn(i32) -> i32 {
    move |x| x + n
}

fn make_counter() -> impl FnMut() -> u32 {
    let mut c = 0;
    move || { c += 1; c }
}

fn compose<A, B, C>(f: impl Fn(A) -> B, g: impl Fn(B) -> C) -> impl Fn(A) -> C {
    move |x| g(f(x))
}

fn evens_squared(limit: u32) -> impl Iterator<Item = u32> {
    (0..limit).filter(|x| x % 2 == 0).map(|x| x * x)
}

fn words<'a>(s: &'a str) -> impl Iterator<Item = &'a str> + 'a {
    s.split(',').map(|w| w.trim()).filter(|w| !w.is_empty())
}

fn apply_twice(f: impl Fn(i32) -> i32, x: i32) -> i32 { f(f(x)) }

fn boxed_ops() -> Vec<(&'static str, Box<dyn Fn(i32, i32) -> i32>)> {
    vec![
        ("add", Box::new(|a, b| a + b)),
        ("sub", Box::new(|a, b| a - b)),
        ("mul", Box::new(move |a, b| a * b)),
    ]
}

fn chain_iters(a: Vec<i32>, b: Vec<i32>) -> impl Iterator<Item = i32> {
    a.into_iter().chain(b.into_iter().rev()).enumerate().map(|(i, x)| x * i as i32)
}

fn make_fib() -> impl Iterator<Item = u64> {
    let mut st = (0u64, 1u64);
    std::iter::from_fn(move || {
        let r = st.0;
        st = (st.1, st.0 + st.1);
        Some(r)
    })
}

fn sum_iter(it: impl IntoIterator<Item = i32>) -> i32 { it.into_iter().sum() }

fn choose(flag: bool) -> Box<dyn Fn(&str) -> String> {
    if flag { Box::new(|s| s.to_uppercase()) } else { Box::new(|s| s.chars().rev().collect()) }
}

fn main() {
    let add5 = make_adder(5);
    println!("{}", add5(10));
    let mut c = make_counter();
    println!("{} {} {}", c(), c(), c());
    let f = compose(|x: i32| x * 3, |y: i32| format!("<{}>", y));
    println!("{}", f(7));
    let g = compose(make_adder(1), make_adder(10));
    println!("{}", g(0));
    println!("{:?}", evens_squared(10).collect::<Vec<_>>());
    println!("{:?}", words("a, b,, c ,d").collect::<Vec<_>>());
    println!("{}", apply_twice(make_adder(3), 1));
    println!("{}", apply_twice(|x| x * x, 3));
    for (n, op) in boxed_ops() { println!("{} {}", n, op(7, 3)); }
    println!("{:?}", chain_iters(vec![1, 2], vec![3, 4, 5]).collect::<Vec<_>>());
    println!("{:?}", make_fib().take(12).collect::<Vec<_>>());
    println!("{}", make_fib().skip_while(|&x| x < 100).next().unwrap());
    println!("{}", sum_iter(vec![1, 2, 3]));
    println!("{}", sum_iter(1..=4));
    println!("{}", sum_iter([5, 6].iter().copied()));
    println!("{} {}", choose(true)("abc"), choose(false)("abc"));
}
