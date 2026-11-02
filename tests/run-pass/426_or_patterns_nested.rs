// Or-patterns in nested positions: inside tuples, enum fields, Option/Result, slices, let bindings,
// function params, closures, with bindings shared across alternatives, and in matches!/if-let.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Suit {
    Hearts,
    Diamonds,
    Clubs,
    Spades,
}

#[derive(Debug, Clone, Copy)]
enum Card {
    Num(u8, Suit),
    Face(char, Suit),
    Joker,
}

#[derive(Debug)]
enum Expr {
    Lit(i64),
    Neg(i64),
    Add(i64, i64),
    Sub(i64, i64),
}

fn color(c: Card) -> &'static str {
    match c {
        Card::Num(_, Suit::Hearts | Suit::Diamonds) | Card::Face(_, Suit::Hearts | Suit::Diamonds) => "red",
        Card::Num(_, Suit::Clubs | Suit::Spades) | Card::Face(_, Suit::Clubs | Suit::Spades) => "black",
        Card::Joker => "none",
    }
}

fn value(c: Card) -> u32 {
    match c {
        Card::Num(n @ (2..=10), _) => n as u32,
        Card::Num(1, _) | Card::Face('A', _) => 11,
        Card::Face('K' | 'Q' | 'J', _) => 10,
        Card::Num(..) | Card::Face(..) => 0,
        Card::Joker => 50,
    }
}

fn eval(e: &Expr) -> i64 {
    match *e {
        Expr::Lit(n) | Expr::Neg(n) if n == 0 => 0,
        Expr::Lit(n) => n,
        Expr::Neg(n) => -n,
        Expr::Add(a, b) | Expr::Sub(b, a) if a == b => a * 2 - if matches!(e, Expr::Sub(..)) { a * 2 } else { 0 },
        Expr::Add(a, b) => a + b,
        Expr::Sub(a, b) => a - b,
    }
}

#[derive(Clone, Copy)]
enum Side {
    L(i32),
    R(i32),
}

fn side_val((Side::L(x) | Side::R(x)): Side) -> i32 {
    x
}

fn main() {
    let cards = [
        Card::Num(7, Suit::Hearts),
        Card::Num(1, Suit::Spades),
        Card::Face('K', Suit::Diamonds),
        Card::Face('A', Suit::Clubs),
        Card::Face('X', Suit::Clubs),
        Card::Num(12, Suit::Diamonds),
        Card::Joker,
    ];
    for c in cards {
        println!("{:?}: {} {}", c, color(c), value(c));
    }
    let reds = cards.iter().filter(|c| matches!(c, Card::Num(_, Suit::Hearts | Suit::Diamonds) | Card::Face(_, Suit::Hearts | Suit::Diamonds))).count();
    println!("reds {}", reds);

    for e in [Expr::Lit(0), Expr::Lit(5), Expr::Neg(3), Expr::Add(2, 2), Expr::Sub(4, 4), Expr::Add(1, 9), Expr::Sub(1, 9)] {
        println!("{:?} = {}", e, eval(&e));
    }

    println!("{} {}", side_val(Side::L(1)), side_val(Side::R(9)));

    let results: Vec<Result<u32, String>> = vec![Ok(3), Err("4".into()), Ok(100), Err("x".into())];
    for r in &results {
        let n = match r {
            Ok(v @ (0..=9 | 100)) => format!("ok special {}", v),
            Ok(v) => format!("ok {}", v),
            Err(s) if s.parse::<u32>().is_ok() => format!("err numeric {}", s),
            Err(_) => "err".to_string(),
        };
        println!("{}", n);
    }

    // or-pattern in let with irrefutable alternatives over a tuple of enums
    let pairs: [(Result<i32, i32>, char); 3] = [(Ok(5), 'a'), (Err(6), 'b'), (Ok(-1), 'c')];
    for (Ok(v) | Err(v), tag) in pairs {
        print!("{}{} ", tag, v);
    }
    println!();

    // nested slices with or
    let seqs: [&[u8]; 5] = [&[1, 2], &[2, 1], &[0], &[3, 3, 3], &[]];
    for s in seqs {
        let d = match s {
            [1, x] | [x, 1] => format!("has one with {}", x),
            [0] | [] => "zeroish".to_string(),
            [a, .., b] | [a @ b] if a == b => format!("same ends {}", a),
            _ => "other".to_string(),
        };
        println!("{:?}: {}", s, d);
    }

    // if-let with or pattern binding
    let maybe: Option<(char, u8)> = Some(('y', 3));
    if let Some(('y' | 'Y', n) | ('n' | 'N', n)) = maybe {
        println!("answer with {}", n);
    }
    let code = 404u16;
    let kind = match code {
        200 | 201 | 204 => "success",
        301 | 302 | 307 | 308 => "redirect",
        400..=499 if code == 404 || code == 410 => "gone",
        400..=499 => "client",
        _ => "server",
    };
    println!("{} {}", code, kind);
    let f = |(Ok(a) | Err(a)): Result<&str, &str>| a.len();
    println!("{} {}", f(Ok("abc")), f(Err("de")));
}
