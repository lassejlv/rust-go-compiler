#[derive(Debug, Clone, PartialEq)]
enum Tok { Num(i64), Op(char), LParen, RParen, Ident(String) }

fn classify(v: &[i32]) -> String {
    match v {
        [] => "empty".into(),
        [x] => format!("one({})", x),
        [a, b] if a == b => format!("pair-eq({})", a),
        [first, .., last] if first == last => format!("bookends({}, len={})", first, v.len()),
        [first, rest @ ..] if rest.iter().all(|r| r > first) => format!("min-first({}) rest={:?}", first, rest),
        [.., x, y, z] => format!("tail3({},{},{})", x, y, z),
        [a, b] => format!("pair({},{})", a, b),
    }
}

fn sum_pairs(v: &[(i32, i32)]) -> i32 {
    let mut total = 0;
    for &(a, b) in v {
        total += a * b;
    }
    for (a, b) in v.iter() {
        // match ergonomics: a, b are &i32
        total += *a - *b;
    }
    total
}

fn eval_prefix(toks: &[Tok]) -> Option<(i64, &[Tok])> {
    match toks {
        [Tok::Num(n), rest @ ..] => Some((*n, rest)),
        [Tok::Op(op), rest @ ..] => {
            let (a, rest) = eval_prefix(rest)?;
            let (b, rest) = eval_prefix(rest)?;
            let v = match op {
                '+' => a + b,
                '-' => a - b,
                '*' => a * b,
                '/' if b != 0 => a / b,
                _ => return None,
            };
            Some((v, rest))
        }
        [Tok::LParen, inner @ .., Tok::RParen] => eval_prefix(inner),
        [Tok::Ident(name), rest @ ..] => {
            let v = match name.as_str() { "ten" => 10, "two" => 2, _ => 0 };
            Some((v, rest))
        }
        _ => None,
    }
}

struct Point3 { x: i32, y: i32, z: i32 }

fn octant(p: &Point3) -> &'static str {
    match p {
        Point3 { x: 0, y: 0, z: 0 } => "origin",
        Point3 { x, y, .. } if *x > 0 && *y > 0 => "q1-ish",
        &Point3 { z, .. } if z < 0 => "below",
        Point3 { x: 1..=5, .. } => "x small",
        _ => "other",
    }
}

fn mutate(v: &mut [i32]) {
    match v {
        [first, .., last] => { std::mem::swap(first, last); }
        _ => {}
    }
    if let [_, second, ..] = v {
        *second *= 100;
    }
    for x in v.iter_mut() {
        if let 1..=9 = *x { *x = -*x; }
    }
}

fn arrays() {
    let arr = [1u8, 2, 3, 4, 5, 6];
    let [a, b, rest @ ..] = arr;
    println!("a={} b={} rest={:?}", a, b, rest);
    let [.., y, z] = arr;
    println!("y={} z={}", y, z);
    if let [1, mid @ .., 6] = &arr[..] {
        println!("mid={:?}", mid);
    }
    let nested = [[1, 2], [3, 4], [5, 6]];
    let [[p, _], .., [_, q]] = nested;
    println!("p={} q={}", p, q);
}

fn main() {
    let cases: Vec<Vec<i32>> = vec![vec![], vec![7], vec![3, 3], vec![4, 1, 2, 4], vec![1, 5, 6, 9], vec![9, 1, 8, 2], vec![5, 2]];
    for c in &cases {
        println!("{:?} -> {}", c, classify(c));
    }
    println!("sum_pairs = {}", sum_pairs(&[(1, 2), (3, 4), (5, -6)]));

    let toks = vec![Tok::Op('+'), Tok::Num(3), Tok::Op('*'), Tok::Ident("ten".into()), Tok::Num(4)];
    println!("{:?}", eval_prefix(&toks).map(|(v, r)| (v, r.len())));
    let toks2 = vec![Tok::LParen, Tok::Op('-'), Tok::Num(100), Tok::Op('/'), Tok::Num(9), Tok::Ident("two".into()), Tok::RParen];
    println!("{:?}", eval_prefix(&toks2).map(|(v, _)| v));
    let toks3 = vec![Tok::Op('/'), Tok::Num(1), Tok::Num(0)];
    println!("{:?}", eval_prefix(&toks3));

    for p in [Point3 { x: 0, y: 0, z: 0 }, Point3 { x: 2, y: 3, z: 1 }, Point3 { x: -1, y: 3, z: -4 }, Point3 { x: 3, y: -3, z: 0 }, Point3 { x: 9, y: -1, z: 2 }] {
        println!("({},{},{}) {}", p.x, p.y, p.z, octant(&p));
    }

    let mut v = vec![1, 2, 30, 4, 50];
    mutate(&mut v);
    println!("mutated {:?}", v);
    let mut single = [5];
    mutate(&mut single);
    println!("single {:?}", single);

    arrays();

    let opt_pair: Option<(String, Vec<u8>)> = Some(("hi".to_string(), vec![1, 2, 3]));
    if let Some((ref s, ref bytes)) = opt_pair {
        println!("{} {:?}", s, bytes);
    }
    match &opt_pair {
        Some((s, b)) if b.len() > 2 => println!("long {} {}", s.len(), b[2]),
        Some(_) => println!("short"),
        None => println!("none"),
    }
    let words = ["alpha", "beta", "gamma", "delta"];
    let mut iter = words.iter();
    while let Some(&w) = iter.next() {
        let tag = match w.as_bytes() {
            [b'a', ..] => "starts-a",
            [.., b'a'] => "ends-a",
            _ => "?",
        };
        println!("{w}: {tag}");
    }
}
