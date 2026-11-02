#[derive(Debug)]
enum Expr {
    Num(i64),
    Add(Box<Expr>, Box<Expr>),
    Neg(Box<Expr>),
}

struct Point3 {
    x: i32,
    y: i32,
    z: i32,
}

fn classify_age(age: u32) -> String {
    match age {
        n @ 0..=12 => format!("child of {}", n),
        n @ 13..=19 => format!("teen of {}", n),
        n @ (20 | 30 | 40) => format!("round adult {}", n),
        n => format!("adult of {}", n),
    }
}

fn simplify(e: &Expr) -> String {
    match e {
        Expr::Num(0) => "zero".to_string(),
        Expr::Num(n) if *n < 0 => format!("negative {}", -n),
        Expr::Num(n) => format!("num {}", n),
        Expr::Add(a, b) => match (a.as_ref(), b.as_ref()) {
            (Expr::Num(0), other) | (other, Expr::Num(0)) => format!("just ({})", simplify(other)),
            (Expr::Num(x), Expr::Num(y)) => format!("const {}", x + y),
            _ => "complex add".to_string(),
        },
        Expr::Neg(inner) => match inner.as_ref() {
            Expr::Neg(x) => format!("double neg of ({})", simplify(x)),
            other => format!("neg of ({})", simplify(other)),
        },
    }
}

fn main() {
    for a in [5, 15, 30, 33] {
        println!("{}", classify_age(a));
    }
    let exprs = vec![
        Expr::Num(0),
        Expr::Num(-4),
        Expr::Num(9),
        Expr::Add(Box::new(Expr::Num(0)), Box::new(Expr::Num(7))),
        Expr::Add(Box::new(Expr::Num(3)), Box::new(Expr::Num(0))),
        Expr::Add(Box::new(Expr::Num(3)), Box::new(Expr::Num(4))),
        Expr::Add(Box::new(Expr::Neg(Box::new(Expr::Num(1)))), Box::new(Expr::Num(4))),
        Expr::Neg(Box::new(Expr::Neg(Box::new(Expr::Num(2))))),
        Expr::Neg(Box::new(Expr::Num(8))),
    ];
    for e in &exprs {
        println!("{:?} => {}", e, simplify(e));
    }

    let p = Point3 { x: 1, y: 0, z: -3 };
    match p {
        Point3 { x: 0, .. } => println!("on yz plane"),
        Point3 { y: 0, z, .. } if z < 0 => println!("y=0 and z negative: {}", z),
        Point3 { x, y, z } => println!("general {} {} {}", x, y, z),
    }

    let arr = [1, 2, 3, 4, 5];
    match arr {
        [first, .., last] => println!("first = {}, last = {}", first, last),
    }
    let slice: &[i32] = &arr[1..];
    match slice {
        [] => println!("empty"),
        [one] => println!("one {}", one),
        [a, b, rest @ ..] => println!("a={} b={} rest={:?}", a, b, rest),
    }

    let pair = (3, "three");
    let msg = match pair {
        (1, _) => "one".to_string(),
        (n, name) if n % 2 == 1 => format!("odd {} named {}", n, name),
        _ => "other".to_string(),
    };
    println!("{}", msg);

    let r = &Some(5);
    if let Some(v) = r {
        println!("ref binding: {}", v);
    }
    let mut maybe = Some(10);
    if let Some(ref mut v) = maybe {
        *v += 1;
    }
    println!("maybe = {:?}", maybe);
}
