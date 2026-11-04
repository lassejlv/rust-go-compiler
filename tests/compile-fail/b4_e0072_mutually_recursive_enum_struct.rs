//~ ERROR E0072
// Mutual recursion without indirection: Expr -> Binary -> Expr.
enum Expr {
    Num(i64),
    Bin(Binary),
}
struct Binary {
    op: char,
    lhs: Expr,
    rhs: Box<Expr>,
}
fn main() {
    let e = Expr::Num(1);
    if let Expr::Num(n) = e { println!("{}", n); }
}
