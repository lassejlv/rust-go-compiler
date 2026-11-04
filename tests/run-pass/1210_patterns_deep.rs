#[derive(Debug, Clone)]
enum Tok {
    Num(i64),
    Op(char),
    Ident(String),
    Group(Vec<Tok>),
}

#[derive(Debug)]
struct Point3 {
    x: i32,
    y: i32,
    z: i32,
}

fn classify(v: &[i32]) -> String {
    match v {
        [] => "empty".into(),
        [x] => format!("one({})", x),
        [first, .., last] if first == last => format!("bookends({})", first),
        [a, b] => format!("two({},{})", a, b),
        [first, mid @ .., last] if mid.len() == 1 => format!("three({} [{}] {})", first, mid[0], last),
        [0, rest @ ..] => format!("zero-led rest={:?}", rest),
        [.., 9, _] => "nine second-to-last".into(),
        [_, second, ..] => format!("second={}", second),
    }
}

fn tok_desc(t: &Tok) -> String {
    match t {
        Tok::Num(n @ (0 | 1)) => format!("bit {}", n),
        Tok::Num(n @ 2..=9) => format!("digit {}", n),
        Tok::Num(n) if *n < 0 => format!("neg {}", n),
        Tok::Num(n) => format!("big {}", n),
        Tok::Op(c @ ('+' | '-')) => format!("additive {}", c),
        Tok::Op(c) => format!("op {}", c),
        Tok::Ident(s) if s.starts_with('_') => "private".into(),
        Tok::Ident(s) => format!("ident {}", s),
        Tok::Group(inner) => match inner.as_slice() {
            [] => "empty group".into(),
            [Tok::Num(a), Tok::Op('+' | '*'), Tok::Num(b)] => format!("binop {} {}", a, b),
            [Tok::Group(g), ..] => format!("nested group of {}", g.len()),
            [only] => format!("single [{}]", tok_desc(only)),
            other => format!("group of {}", other.len()),
        },
    }
}

fn points(p: &Point3) -> &'static str {
    match p {
        Point3 { x: 0, y: 0, z: 0 } => "origin",
        Point3 { x: 0, .. } | Point3 { y: 0, .. } => "on a plane-ish",
        Point3 { z: z @ 1..=5, .. } if *z % 2 == 1 => "low odd z",
        &Point3 { x, y, z } if x == y && y == z => "diagonal",
        _ => "elsewhere",
    }
}

fn nested_opt(v: Option<Result<Option<u8>, String>>) -> String {
    match v {
        None => "none".into(),
        Some(Ok(None)) => "ok-none".into(),
        Some(Ok(Some(n @ (1 | 3 | 5..=7)))) => format!("special {}", n),
        Some(Ok(Some(n))) => format!("plain {}", n),
        Some(Err(ref e)) if e.is_empty() => "empty err".into(),
        Some(Err(e)) => format!("err {}", e),
    }
}

fn main() {
    for v in [
        &[][..],
        &[1],
        &[3, 4, 3],
        &[1, 2],
        &[1, 2, 3],
        &[0, 5, 6, 7],
        &[1, 2, 9, 4],
        &[5, 6, 7, 8],
    ] {
        println!("{:?} -> {}", v, classify(v));
    }

    let toks = vec![
        Tok::Num(1),
        Tok::Num(7),
        Tok::Num(-3),
        Tok::Num(100),
        Tok::Op('-'),
        Tok::Op('/'),
        Tok::Ident("_x".into()),
        Tok::Ident("y".into()),
        Tok::Group(vec![]),
        Tok::Group(vec![Tok::Num(2), Tok::Op('*'), Tok::Num(3)]),
        Tok::Group(vec![Tok::Group(vec![Tok::Num(1)]), Tok::Num(1)]),
        Tok::Group(vec![Tok::Ident("z".into())]),
        Tok::Group(vec![Tok::Num(2), Tok::Op('-'), Tok::Num(3)]),
    ];
    for t in &toks {
        println!("{}", tok_desc(t));
    }

    for p in [
        Point3 { x: 0, y: 0, z: 0 },
        Point3 { x: 0, y: 1, z: 0 },
        Point3 { x: 2, y: 2, z: 3 },
        Point3 { x: 4, y: 4, z: 4 },
        Point3 { x: 4, y: 5, z: 6 },
    ] {
        println!("{:?} {}", p, points(&p));
    }

    for v in [None, Some(Ok(None)), Some(Ok(Some(6))), Some(Ok(Some(4))), Some(Err(String::new())), Some(Err("bad".to_string()))] {
        println!("{}", nested_opt(v));
    }

    // default binding modes
    let pairs = vec![(1, "one".to_string()), (2, "two".to_string())];
    for (n, s) in &pairs {
        let _: &i32 = n;
        let _: &String = s;
        print!("{}={} ", n, s);
    }
    println!();
    let mut pairs2 = pairs.clone();
    for (n, s) in &mut pairs2 {
        *n *= 10;
        s.push('!');
    }
    println!("{:?}", pairs2);
    let opt = Some(String::from("hi"));
    if let Some(s) = &opt {
        println!("borrowed {}", s);
    }
    match &opt {
        Some(s) if s.len() > 5 => println!("long"),
        Some(s) => println!("short {}", s.len()),
        None => {}
    }
    let mut mo = Some(5);
    if let Some(x) = &mut mo {
        *x += 1;
    }
    println!("{:?}", mo);
    let r = &&(1, (2, 3));
    let &&(a, (b, c)) = r;
    let (d, (e, f)) = r;
    println!("{} {} {} {} {} {}", a, b, c, d, e, f);

    // binding @ with subpatterns over structs and slices
    let arr = [1, 2, 3, 4, 5];
    if let [first @ 1, rest @ .., last @ 5] = arr {
        println!("{} {:?} {}", first, rest, last);
    }
    let msg = Some(Point3 { x: 1, y: 2, z: 3 });
    if let Some(p @ Point3 { x: 1, .. }) = &msg {
        println!("p.z={}", p.z);
    }
    let v = vec![10, 20, 30];
    match v.as_slice() {
        whole @ [_, tail @ ..] => println!("{:?} {:?}", whole, tail),
        [] => {}
    }
    // matches! and let chains via nested if let
    let ts: Vec<bool> = toks.iter().map(|t| matches!(t, Tok::Num(n) if *n > 5) || matches!(t, Tok::Op('+' | '-'))).collect();
    println!("{:?}", ts);
    let chars = ['a', 'Z', '5', '_', ' '];
    for c in chars {
        let k = match c {
            'a'..='z' | 'A'..='Z' => "alpha",
            '0'..='9' => "digit",
            '_' => "under",
            _ => "other",
        };
        print!("{} ", k);
    }
    println!();
    let x = 42u8;
    let desc = match x {
        0 => "zero",
        1..=9 => "small",
        10 | 20 | 30 | 40 => "round",
        n if n % 2 == 0 => "even",
        _ => "odd",
    };
    println!("{}", desc);
    // tuple struct and box patterns via deref
    struct W(Option<Box<i32>>);
    let w = W(Some(Box::new(7)));
    if let W(Some(ref b)) = w {
        println!("{}", **b + 1);
    }
    let (mut m, n) = (1, 2);
    m += n;
    println!("{}", m);
    // constants in patterns
    const LIMIT: i32 = 10;
    match 10 {
        LIMIT => println!("at limit"),
        _ => println!("not"),
    }
    // i128/char ranges
    match -5i128 {
        i128::MIN..=-1 => println!("negative i128"),
        _ => println!("nonneg"),
    }
    let s = "hello";
    match s {
        "hi" | "hello" => println!("greeting"),
        _ => println!("?"),
    }
    let ostr: Option<&str> = Some("x");
    match ostr {
        Some("x") => println!("x!"),
        _ => {}
    }
}
