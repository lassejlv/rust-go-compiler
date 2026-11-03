// Match ergonomics with &mut: binding modes and nested mutation.
#[derive(Debug)]
enum Expr {
    Num(i64),
    Add(Box<Expr>, Box<Expr>),
    Neg(Box<Expr>),
}

fn fold(e: &mut Expr) {
    match e {
        Expr::Add(a, b) => {
            fold(a);
            fold(b);
            if let (Expr::Num(x), Expr::Num(y)) = (a.as_ref(), b.as_ref()) {
                *e = Expr::Num(x + y);
            }
        }
        Expr::Neg(inner) => {
            fold(inner);
            if let Expr::Num(x) = **inner {
                *e = Expr::Num(-x);
            }
        }
        Expr::Num(n) => *n *= 1,
    }
}

fn main() {
    let mut e = Expr::Add(
        Box::new(Expr::Num(2)),
        Box::new(Expr::Neg(Box::new(Expr::Add(Box::new(Expr::Num(3)), Box::new(Expr::Num(4)))))),
    );
    fold(&mut e);
    println!("{:?}", e);
    let mut pairs = vec![(1, String::from("a")), (2, String::from("b"))];
    for (n, s) in &mut pairs {
        *n *= 10;
        s.push('!');
    }
    println!("{:?}", pairs);
}
