// nested match with guards, @ bindings, bindings in or-patterns, ranges, refs, box-like structures
#[derive(Debug, Clone, PartialEq)]
enum Expr {
    Num(i64),
    Neg(Box<Expr>),
    Add(Box<Expr>, Box<Expr>),
    Mul(Box<Expr>, Box<Expr>),
    Var(String),
}
use Expr::*;

fn simplify(e: Expr) -> Expr {
    match e {
        Add(a, b) => match (simplify(*a), simplify(*b)) {
            (Num(0), x) | (x, Num(0)) => x,
            (Num(x), Num(y)) => Num(x + y),
            (x, y) if x == y => Mul(Box::new(Num(2)), Box::new(x)),
            (x, y) => Add(Box::new(x), Box::new(y)),
        },
        Mul(a, b) => match (simplify(*a), simplify(*b)) {
            (Num(0), _) | (_, Num(0)) => Num(0),
            (Num(1), x) | (x, Num(1)) => x,
            (Num(x), Num(y)) => Num(x * y),
            (x, y) => Mul(Box::new(x), Box::new(y)),
        },
        Neg(inner) => match simplify(*inner) {
            Num(n) => Num(-n),
            Neg(x) => *x,
            other => Neg(Box::new(other)),
        },
        other => other,
    }
}

fn show(e: &Expr) -> String {
    match e {
        Num(n) if *n < 0 => format!("({})", n),
        Num(n) => n.to_string(),
        Var(v) => v.clone(),
        Neg(x) => format!("-{}", show(x)),
        Add(a, b) => format!("({} + {})", show(a), show(b)),
        Mul(a, b) => format!("{} * {}", show(a), show(b)),
    }
}

#[derive(Debug)]
enum Msg {
    Move { x: i32, y: i32 },
    Write(String),
    Color(u8, u8, u8),
    Batch(Vec<Msg>),
    Quit,
}

fn handle(m: &Msg, depth: usize) -> String {
    let pad = "  ".repeat(depth);
    match m {
        Msg::Move { x: 0, y: 0 } => format!("{}stay", pad),
        Msg::Move { x, y: 0 } | Msg::Move { x: 0, y: x } => format!("{}axis move {}", pad, x),
        Msg::Move { x: x @ -10..=10, y: y @ -10..=10 } => format!("{}short move {},{}", pad, x, y),
        Msg::Move { x, y } if x.abs() == y.abs() => format!("{}diagonal {}", pad, x),
        Msg::Move { .. } => format!("{}long move", pad),
        Msg::Write(s) if s.is_empty() => format!("{}empty write", pad),
        Msg::Write(s) => match s.as_str() {
            "hello" | "hi" => format!("{}greeting", pad),
            t @ _ if t.len() > 5 => format!("{}long text {}", pad, &t[..5]),
            t => format!("{}text {}", pad, t),
        },
        &Msg::Color(r, g, b) if r == g && g == b => format!("{}gray {}", pad, r),
        Msg::Color(255, _, _) | Msg::Color(_, 255, _) | Msg::Color(_, _, 255) => format!("{}saturated", pad),
        Msg::Color(r, ..) => format!("{}color r={}", pad, r),
        Msg::Batch(v) if v.is_empty() => format!("{}empty batch", pad),
        Msg::Batch(v) => {
            let inner: Vec<String> = v.iter().map(|m| handle(m, depth + 1)).collect();
            format!("{}batch[{}]\n{}", pad, v.len(), inner.join("\n"))
        }
        Msg::Quit => format!("{}quit", pad),
    }
}

fn bucket(n: u32) -> String {
    match n {
        0 => "zero".into(),
        d @ 1..=9 => format!("digit {}", d),
        t @ (10..=99 | 1000..=1999) => format!("tens-or-thousand {}", t),
        n @ 100..=999 if n % 100 == 0 => format!("round hundred {}", n),
        n @ 100..=999 => format!("hundreds {}", n),
        n => format!("big {}", n),
    }
}

fn main() {
    let e = Add(
        Box::new(Mul(Box::new(Num(1)), Box::new(Var("x".into())))),
        Box::new(Add(Box::new(Num(0)), Box::new(Var("x".into())))),
    );
    println!("{}", show(&e));
    println!("{}", show(&simplify(e)));
    let e2 = Neg(Box::new(Neg(Box::new(Add(Box::new(Num(3)), Box::new(Num(-5)))))));
    println!("{} => {}", show(&e2), show(&simplify(e2.clone())));
    let e3 = Mul(Box::new(Var("y".into())), Box::new(Add(Box::new(Num(2)), Box::new(Num(-2)))));
    println!("{} => {:?}", show(&e3), simplify(e3.clone()));
    let msgs = vec![
        Msg::Move { x: 0, y: 0 },
        Msg::Move { x: 5, y: 0 },
        Msg::Move { x: 0, y: -7 },
        Msg::Move { x: 3, y: 4 },
        Msg::Move { x: -20, y: 20 },
        Msg::Move { x: 50, y: 1 },
        Msg::Write(String::new()),
        Msg::Write("hi".into()),
        Msg::Write("abcdefgh".into()),
        Msg::Write("abc".into()),
        Msg::Color(7, 7, 7),
        Msg::Color(1, 255, 3),
        Msg::Color(9, 8, 7),
        Msg::Batch(vec![Msg::Quit, Msg::Batch(vec![]), Msg::Batch(vec![Msg::Color(0, 0, 255)])]),
    ];
    for m in &msgs {
        println!("{}", handle(m, 0));
    }
    for n in [0, 7, 42, 300, 301, 1500, 5000] {
        println!("{}", bucket(n));
    }
    let pair = (Some(3), Err::<i32, &str>("bad"));
    match pair {
        (Some(n @ 1..=5), Err(msg)) if msg.len() == 3 => println!("matched {} {}", n, msg),
        _ => println!("nope"),
    }
    let v = vec![(1, 'a'), (2, 'b'), (3, 'c')];
    for &(n, ref c) in &v {
        match (n % 2, c) {
            (0, ch) => println!("even {} {}", n, ch),
            (_, 'a'..='b') => println!("odd low {}", n),
            (_, other) => println!("odd other {}", other),
        }
    }
    let nested = Some(Some(Ok::<u8, ()>(5)));
    if let Some(Some(Ok(x @ 0..=9))) = nested {
        println!("deep {}", x);
    }
}
