// @ bindings with ranges and subpatterns, char/u8/i64/negative ranges, exclusive range patterns,
// bindings on struct fields, binding after @ with nested destructuring, and const ranges.
const LOW: i32 = -10;
const HIGH: i32 = 10;

#[derive(Debug)]
struct Point {
    x: i32,
    y: i32,
}

#[derive(Debug)]
enum Token {
    Num(i64),
    Op(char),
    Ident(String),
}

fn bucket(n: i32) -> String {
    match n {
        i32::MIN..=-1001 => "very negative".into(),
        x @ -1000..LOW => format!("negative {}", x),
        x @ LOW..=HIGH => format!("near zero {}", x),
        x @ (11..=99 | 101..=199) => format!("two/three digit {}", x),
        100 => "hundred".into(),
        x @ 200.. => format!("big {}", x),
    }
}

fn char_kind(c: char) -> String {
    match c {
        d @ '0'..='9' => format!("digit {}", d as u32 - '0' as u32),
        l @ ('a'..='z' | 'A'..='Z') => format!("letter {}", l.to_ascii_lowercase()),
        ' ' | '\t' | '\n' => "space".into(),
        g @ 'α'..='ω' => format!("greek {}", g),
        other => format!("other U+{:04X}", other as u32),
    }
}

fn byte_class(b: u8) -> &'static str {
    match b {
        0x00..=0x1F | 0x7F => "control",
        b' ' => "space",
        b'!'..=b'/' | b':'..=b'@' | b'['..=b'`' | b'{'..=b'~' => "punct",
        b'0'..=b'9' => "digit",
        0x80.. => "high",
        _ => "alpha",
    }
}

fn point_region(p: &Point) -> String {
    match p {
        Point { x: 0, y: 0 } => "origin".into(),
        Point { x: px @ 1.., y: 0 } => format!("+x axis at {}", px),
        Point { x: 0, y: py @ ..=-1 } => format!("-y axis at {}", py),
        Point { x: x @ -5..=5, y: y @ -5..=5 } => format!("near ({},{})", x, y),
        whole @ Point { x, .. } if *x > 100 => format!("far right {:?}", whole),
        Point { x, y } => format!("elsewhere {}", x * y),
    }
}

fn token_desc(t: &Token) -> String {
    match t {
        Token::Num(n @ (0 | 1)) => format!("bit {}", n),
        Token::Num(n @ 2..=9) => format!("digit {}", n),
        Token::Num(n) if n % 2 == 0 => format!("even {}", n),
        Token::Num(n) => format!("odd {}", n),
        Token::Op(op @ ('+' | '-')) => format!("additive {}", op),
        Token::Op(op) => format!("op {}", op),
        Token::Ident(name) if name.starts_with('_') => "private".into(),
        Token::Ident(name) => format!("ident {} ({})", name, name.len()),
    }
}

fn main() {
    for n in [-5000, -500, -10, 0, 10, 11, 100, 150, 200, 99999] {
        println!("{} -> {}", n, bucket(n));
    }
    for c in ['7', 'Q', 'z', ' ', 'β', '€', '\u{1F600}'] {
        println!("{:?} -> {}", c, char_kind(c));
    }
    let classes: Vec<&str> = b"Hi, 5!\x01\xff~".iter().map(|&b| byte_class(b)).collect();
    println!("{:?}", classes);
    for p in [
        Point { x: 0, y: 0 },
        Point { x: 7, y: 0 },
        Point { x: 0, y: -3 },
        Point { x: -2, y: 4 },
        Point { x: 200, y: 1 },
        Point { x: 9, y: 9 },
    ] {
        println!("{:?} -> {}", p, point_region(&p));
    }
    let toks = vec![
        Token::Num(1),
        Token::Num(7),
        Token::Num(12),
        Token::Num(-3),
        Token::Op('-'),
        Token::Op('*'),
        Token::Ident("_x".into()),
        Token::Ident("value".into()),
    ];
    for t in &toks {
        println!("{:?}: {}", t, token_desc(t));
    }
    // @ binding whole tuple and parts
    let t = (3, (4, 5));
    let whole @ (a, inner @ (b, c)) = t;
    println!("{:?} {} {:?} {} {}", whole, a, inner, b, c);
    // @ with Option
    let vals = [Some(3), Some(42), None];
    for v in vals {
        let s = match v {
            Some(n @ 1..=9) => format!("small {}", n),
            Some(n) => format!("other {}", n),
            None => "none".into(),
        };
        println!("{}", s);
    }
    // u64 boundaries & i8 full range
    let big: u64 = u64::MAX - 1;
    let r = match big {
        0..=0xFFFF_FFFF => "u32 range",
        0x1_0000_0000..=0xFFFF_FFFF_FFFF_FFFE => "large",
        u64::MAX => "max",
    };
    println!("{}", r);
    let s: i8 = -128;
    println!("{}", match s { i8::MIN => "min", -127..=-1 => "neg", 0 => "zero", 1..=i8::MAX => "pos" });
}
