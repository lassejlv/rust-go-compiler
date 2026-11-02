#[derive(Debug)]
enum Expr {
    Num(i64),
    Neg(Box<Expr>),
    Add(Box<Expr>, Box<Expr>),
    Mul(Box<Expr>, Box<Expr>),
}

use Expr::*;

fn simplify(e: Expr) -> Expr {
    match e {
        Add(a, b) => match (simplify(*a), simplify(*b)) {
            (Num(0), x) | (x, Num(0)) => x,
            (Num(x), Num(y)) => Num(x + y),
            (x, y) => Add(Box::new(x), Box::new(y)),
        },
        Mul(a, b) => match (simplify(*a), simplify(*b)) {
            (Num(0), _) | (_, Num(0)) => Num(0),
            (Num(1), x) | (x, Num(1)) => x,
            (Num(x), Num(y)) => Num(x * y),
            (x, y) => Mul(Box::new(x), Box::new(y)),
        },
        Neg(inner) => match simplify(*inner) {
            Neg(x) => *x,
            Num(n) => Num(-n),
            other => Neg(Box::new(other)),
        },
        n => n,
    }
}

fn age_group(age: u32) -> String {
    match age {
        0 => "newborn".to_string(),
        n @ 1..=12 => format!("child({})", n),
        n @ (13..=19) => format!("teen({})", n),
        n @ 20..=64 if n % 10 == 0 => format!("round adult({})", n),
        20..=64 => "adult".to_string(),
        n => format!("senior({})", n),
    }
}

fn slice_desc(v: &[i32]) -> String {
    match v {
        [] => "empty".into(),
        [x] => format!("one {}", x),
        [first, .., last] if first == last => format!("same ends {}", first),
        [first, middle @ .., last] => format!("{} [{} mid, sum {}] {}", first, middle.len(), middle.iter().sum::<i32>(), last),
    }
}

fn head_tail(v: &[&str]) -> String {
    match v {
        ["start", rest @ ..] => format!("starts; rest={:?}", rest),
        [.., "end"] => "ends".into(),
        [a, b, ..] if a.len() == b.len() => format!("same len {} {}", a, b),
        _ => "other".into(),
    }
}

fn pair_refs(a: &Option<i32>, b: &Option<i32>) -> i32 {
    match (a, b) {
        (Some(x), Some(y)) if x > y => x - y,
        (Some(x), Some(y)) => y - x,
        (Some(x), None) | (None, Some(x)) => *x,
        (None, None) => -1,
    }
}

fn shadow(v: Option<(i32, &str)>) -> String {
    match v {
        Some((n, s)) if n > 0 => {
            let n = n * 2;
            let s = s.to_uppercase();
            format!("{}{}", s, n)
        }
        Some((n, _)) => {
            let n = -n;
            format!("flipped {}", n)
        }
        None => String::from("none"),
    }
}

fn main() {
    let e = Add(Box::new(Mul(Box::new(Num(1)), Box::new(Num(7)))), Box::new(Neg(Box::new(Neg(Box::new(Num(0)))))));
    println!("{:?}", simplify(e));
    let e2 = Mul(Box::new(Add(Box::new(Num(2)), Box::new(Num(3)))), Box::new(Neg(Box::new(Num(4)))));
    println!("{:?}", simplify(e2));
    let e3 = Neg(Box::new(Add(Box::new(Num(0)), Box::new(Neg(Box::new(Mul(Box::new(Num(5)), Box::new(Num(0)))))))));
    println!("{:?}", simplify(e3));

    for a in [0, 5, 15, 30, 33, 70] {
        println!("{}", age_group(a));
    }
    for v in [&[][..], &[4], &[3, 9, 3], &[1, 2, 3, 4, 5], &[1, 2]] {
        println!("{}", slice_desc(v));
    }
    println!("{}", head_tail(&["start", "a", "b"]));
    println!("{}", head_tail(&["x", "end"]));
    println!("{}", head_tail(&["ab", "cd", "e"]));
    println!("{}", head_tail(&["abc", "d"]));
    println!("{} {} {} {}", pair_refs(&Some(9), &Some(4)), pair_refs(&Some(2), &Some(8)), pair_refs(&None, &Some(6)), pair_refs(&None, &None));
    println!("{} {} {}", shadow(Some((3, "ab"))), shadow(Some((-4, "q"))), shadow(None));

    let msg = match (3, "x", 'c') {
        (1 | 2, _, _) => "small",
        (3, "y", _) => "three-y",
        (3, _, 'a'..='d') => "three-letter",
        _ => "default",
    };
    println!("{}", msg);
    let nested = Some(Ok::<i32, String>(4));
    if let Some(Ok(n @ 1..=9)) = nested {
        println!("digit {}", n);
    }
    let point = (3, -3);
    match point {
        (x, y) if x == y => println!("diag"),
        (x, y) if x == -y => println!("anti-diag {}", x),
        _ => println!("off"),
    }
    let c = 'Z';
    let kind = match c {
        'a'..='z' => "lower",
        'A'..='Y' => "upper-not-z",
        'Z' | 'z' => "zed",
        _ => "other",
    };
    println!("{}", kind);
}
