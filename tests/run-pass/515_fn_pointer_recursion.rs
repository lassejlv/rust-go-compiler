// recursion via fn pointers, fn pointer tables, fn items coerced, Y-combinator-ish
type Op = fn(i64, i64) -> i64;

fn add(a: i64, b: i64) -> i64 {
    a + b
}
fn sub(a: i64, b: i64) -> i64 {
    a - b
}
fn mul(a: i64, b: i64) -> i64 {
    a * b
}

const OPS: [(char, Op); 3] = [('+', add), ('-', sub), ('*', mul)];

fn lookup(c: char) -> Option<Op> {
    OPS.iter().find(|(k, _)| *k == c).map(|(_, f)| *f)
}

fn fact(f: fn(u64) -> u64, n: u64) -> u64 {
    if n == 0 { 1 } else { n * f(n - 1) }
}
fn fact_rec(n: u64) -> u64 {
    fact(fact_rec, n)
}

struct Rec<'a> {
    f: &'a dyn Fn(&Rec, u64) -> u64,
}
fn fix(n: u64) -> u64 {
    let fibf = |r: &Rec, n: u64| -> u64 {
        if n < 2 { n } else { (r.f)(r, n - 1) + (r.f)(r, n - 2) }
    };
    let r = Rec { f: &fibf };
    (r.f)(&r, n)
}

fn is_even(n: u32) -> bool {
    if n == 0 { true } else { (ODD)(n - 1) }
}
fn is_odd(n: u32) -> bool {
    if n == 0 { false } else { (EVEN)(n - 1) }
}
static EVEN: fn(u32) -> bool = is_even;
static ODD: fn(u32) -> bool = is_odd;

fn apply_twice(f: fn(i32) -> i32, x: i32) -> i32 {
    f(f(x))
}
fn inc(x: i32) -> i32 {
    x + 1
}

fn ackermann(m: u64, n: u64) -> u64 {
    let f: fn(u64, u64) -> u64 = ackermann;
    match (m, n) {
        (0, n) => n + 1,
        (m, 0) => f(m - 1, 1),
        (m, n) => f(m - 1, f(m, n - 1)),
    }
}

fn hanoi(n: u32, from: char, to: char, via: char, moves: &mut Vec<(char, char)>) {
    if n == 0 {
        return;
    }
    hanoi(n - 1, from, via, to, moves);
    moves.push((from, to));
    hanoi(n - 1, via, to, from, moves);
}

fn eval_rpn(tokens: &[&str]) -> Option<i64> {
    let mut st = vec![];
    for t in tokens {
        if let Ok(n) = t.parse::<i64>() {
            st.push(n);
        } else {
            let op = lookup(t.chars().next()?)?;
            let b = st.pop()?;
            let a = st.pop()?;
            st.push(op(a, b));
        }
    }
    if st.len() == 1 { st.pop() } else { None }
}

fn main() {
    for (c, f) in OPS.iter() {
        println!("{} -> {}", c, f(7, 3));
    }
    println!("{:?}", lookup('/').is_none());
    println!("{}", fact_rec(15));
    println!("{}", fix(20));
    println!("{} {} {}", is_even(10), is_odd(7), is_even(3));
    println!("{}", apply_twice(inc, 5));
    println!("{}", apply_twice(|x| x * 3, 5));
    let fs: Vec<fn(i32) -> i32> = vec![inc, |x| x - 1, i32::abs, i32::signum];
    println!("{:?}", fs.iter().map(|f| f(-9)).collect::<Vec<_>>());
    println!("{}", ackermann(2, 3));
    println!("{}", ackermann(3, 3));
    let mut moves = vec![];
    hanoi(4, 'A', 'C', 'B', &mut moves);
    println!("{} {:?}", moves.len(), &moves[..5]);
    println!("{:?}", eval_rpn(&["3", "4", "+", "2", "*", "10", "-"]));
    println!("{:?}", eval_rpn(&["1", "+"]));
    let p: fn(i64, i64) -> i64 = add;
    let q = p;
    println!("{}", p == q);
    let method: fn(&str) -> String = str::to_uppercase;
    println!("{}", method("shout"));
    let ctor: fn(i32) -> Option<i32> = Some;
    println!("{:?}", [1, 2].map(ctor));
}
