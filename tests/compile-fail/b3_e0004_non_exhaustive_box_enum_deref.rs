//~ ERROR E0004
enum Expr { Num(i64), Add(Box<Expr>, Box<Expr>), Neg(Box<Expr>) }
fn eval(e: &Expr) -> i64 {
    match e {
        Expr::Num(n) => *n,
        Expr::Add(a, b) => eval(a) + eval(b),
    }
}
fn main() { eval(&Expr::Num(1)); }
