fn apply<F: Fn(i32) -> i32>(f: F, x: i32) -> i32 {
    f(x)
}

fn apply_twice(f: impl Fn(i32) -> i32, x: i32) -> i32 {
    f(f(x))
}

fn call_n<F>(mut f: F, n: usize)
where
    F: FnMut(usize),
{
    for i in 0..n {
        f(i);
    }
}

fn consume_once<F: FnOnce() -> String>(f: F) -> String {
    let s = f();
    format!("consumed: {}", s)
}

fn apply_dyn(f: &dyn Fn(f64) -> f64, x: f64) -> f64 {
    f(x)
}

fn apply_mut_ref(f: &mut dyn FnMut() -> u32) -> u32 {
    f() + f()
}

fn double(x: i32) -> i32 {
    x * 2
}

fn transform_all(v: &[i32], f: fn(i32) -> i32) -> Vec<i32> {
    v.iter().map(|&x| f(x)).collect()
}

fn main() {
    let offset = 10;
    println!("apply = {}", apply(|x| x + offset, 5));
    println!("apply fn item = {}", apply(double, 21));
    println!("apply_twice = {}", apply_twice(|x| x * 3, 2));
    let mut log = Vec::new();
    call_n(|i| log.push(i * i), 5);
    println!("log = {:?}", log);
    let mut sum = 0;
    call_n(|i| sum += i, 101);
    println!("sum = {}", sum);
    let owned = String::from("treasure");
    println!("{}", consume_once(move || owned + "!"));
    println!("{}", consume_once(|| "literal".to_string()));
    println!("apply_dyn = {}", apply_dyn(&|x| x.sqrt(), 81.0));
    let base = 1.5;
    println!("apply_dyn capture = {}", apply_dyn(&|x| x * base, 4.0));
    let mut c = 0;
    let mut counter = || {
        c += 5;
        c
    };
    println!("apply_mut_ref = {}", apply_mut_ref(&mut counter));
    println!("c = {}", c);
    println!("fn pointer = {:?}", transform_all(&[1, 2, 3], double));
    println!("closure as fn pointer = {:?}", transform_all(&[1, 2, 3], |x| x - 1));
    let ops: Vec<(&str, fn(i32, i32) -> i32)> = vec![
        ("add", |a, b| a + b),
        ("sub", |a, b| a - b),
        ("mul", |a, b| a * b),
        ("max", std::cmp::max),
    ];
    for (name, op) in &ops {
        println!("{}(7, 3) = {}", name, op(7, 3));
    }
}
