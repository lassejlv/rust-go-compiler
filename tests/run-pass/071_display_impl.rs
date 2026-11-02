use std::fmt;

struct Point {
    x: i32,
    y: i32,
}

impl fmt::Display for Point {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "({}, {})", self.x, self.y)
    }
}

struct Matrix2 {
    a: [[f64; 2]; 2],
}

impl fmt::Display for Matrix2 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "| {:6.2} {:6.2} |", self.a[0][0], self.a[0][1])?;
        write!(f, "| {:6.2} {:6.2} |", self.a[1][0], self.a[1][1])
    }
}

enum Token {
    Num(f64),
    Op(char),
    LParen,
    RParen,
}

impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Token::Num(n) => write!(f, "{}", n),
            Token::Op(c) => write!(f, "{}", c),
            Token::LParen => write!(f, "("),
            Token::RParen => write!(f, ")"),
        }
    }
}

struct List(Vec<i32>);

impl fmt::Display for List {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "[")?;
        for (i, v) in self.0.iter().enumerate() {
            if i != 0 {
                write!(f, ", ")?;
            }
            write!(f, "{}", v)?;
        }
        write!(f, "]")
    }
}

struct Name(String);

impl fmt::Display for Name {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.pad(&self.0)
    }
}

#[derive(Debug)]
struct Both {
    v: i32,
}

impl fmt::Display for Both {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Both={}", self.v)
    }
}

fn main() {
    let p = Point { x: 3, y: -7 };
    println!("{}", p);
    println!("point is {} !", p);
    let s = p.to_string();
    println!("to_string len = {}", s.len());
    let m = Matrix2 { a: [[1.0, 2.5], [-3.25, 100.0]] };
    println!("{}", m);
    let toks = vec![Token::LParen, Token::Num(1.5), Token::Op('+'), Token::Num(2.0), Token::RParen, Token::Op('*'), Token::Num(3.0)];
    let rendered: Vec<String> = toks.iter().map(|t| t.to_string()).collect();
    println!("{}", rendered.join(" "));
    println!("{}", List(vec![1, 2, 3]));
    println!("{}", List(vec![]));
    println!("[{:>8}] [{:<8}] [{:^8}]", Name("bob".into()), Name("al".into()), Name("x".into()));
    let b = Both { v: 9 };
    println!("{} / {:?}", b, b);
    let formatted = format!("{}-{}", Point { x: 0, y: 0 }, List(vec![5]));
    println!("{}", formatted);
}
