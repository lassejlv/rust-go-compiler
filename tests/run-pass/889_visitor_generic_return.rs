#[derive(Debug, Clone)]
enum Expr {
    Num(f64),
    Var(String),
    Bin(Box<Expr>, char, Box<Expr>),
    Neg(Box<Expr>),
    Let(String, Box<Expr>, Box<Expr>),
}

trait Visitor {
    type Out;
    fn num(&mut self, n: f64) -> Self::Out;
    fn var(&mut self, name: &str) -> Self::Out;
    fn bin(&mut self, l: &Expr, op: char, r: &Expr) -> Self::Out;
    fn neg(&mut self, e: &Expr) -> Self::Out;
    fn let_(&mut self, name: &str, v: &Expr, body: &Expr) -> Self::Out;
}

impl Expr {
    fn accept<V: Visitor>(&self, v: &mut V) -> V::Out {
        match self {
            Expr::Num(n) => v.num(*n),
            Expr::Var(s) => v.var(s),
            Expr::Bin(l, op, r) => v.bin(l, *op, r),
            Expr::Neg(e) => v.neg(e),
            Expr::Let(n, val, body) => v.let_(n, val, body),
        }
    }
}

struct Printer;
impl Visitor for Printer {
    type Out = String;
    fn num(&mut self, n: f64) -> String { format!("{}", n) }
    fn var(&mut self, name: &str) -> String { name.to_string() }
    fn bin(&mut self, l: &Expr, op: char, r: &Expr) -> String { format!("({} {} {})", l.accept(self), op, r.accept(self)) }
    fn neg(&mut self, e: &Expr) -> String { format!("-{}", e.accept(self)) }
    fn let_(&mut self, n: &str, v: &Expr, b: &Expr) -> String { format!("let {} = {} in {}", n, v.accept(self), b.accept(self)) }
}

struct Eval { env: Vec<(String, f64)> }
impl Visitor for Eval {
    type Out = Result<f64, String>;
    fn num(&mut self, n: f64) -> Self::Out { Ok(n) }
    fn var(&mut self, name: &str) -> Self::Out {
        self.env.iter().rev().find(|(k, _)| k == name).map(|(_, v)| *v).ok_or_else(|| format!("unbound {}", name))
    }
    fn bin(&mut self, l: &Expr, op: char, r: &Expr) -> Self::Out {
        let a = l.accept(self)?;
        let b = r.accept(self)?;
        match op {
            '+' => Ok(a + b),
            '-' => Ok(a - b),
            '*' => Ok(a * b),
            '/' => if b == 0.0 { Err("div by zero".into()) } else { Ok(a / b) },
            _ => Err(format!("bad op {}", op)),
        }
    }
    fn neg(&mut self, e: &Expr) -> Self::Out { Ok(-e.accept(self)?) }
    fn let_(&mut self, n: &str, v: &Expr, b: &Expr) -> Self::Out {
        let val = v.accept(self)?;
        self.env.push((n.to_string(), val));
        let r = b.accept(self);
        self.env.pop();
        r
    }
}

struct Counter { nodes: usize, vars: Vec<String> }
impl Visitor for Counter {
    type Out = ();
    fn num(&mut self, _: f64) { self.nodes += 1; }
    fn var(&mut self, n: &str) { self.nodes += 1; self.vars.push(n.to_string()); }
    fn bin(&mut self, l: &Expr, _: char, r: &Expr) { self.nodes += 1; l.accept(self); r.accept(self); }
    fn neg(&mut self, e: &Expr) { self.nodes += 1; e.accept(self) }
    fn let_(&mut self, _: &str, v: &Expr, b: &Expr) { self.nodes += 1; v.accept(self); b.accept(self); }
}

struct Depth;
impl Visitor for Depth {
    type Out = usize;
    fn num(&mut self, _: f64) -> usize { 1 }
    fn var(&mut self, _: &str) -> usize { 1 }
    fn bin(&mut self, l: &Expr, _: char, r: &Expr) -> usize { 1 + l.accept(self).max(r.accept(self)) }
    fn neg(&mut self, e: &Expr) -> usize { 1 + e.accept(self) }
    fn let_(&mut self, _: &str, v: &Expr, b: &Expr) -> usize { 1 + v.accept(self).max(b.accept(self)) }
}

fn run_all<V: Visitor>(v: &mut V, es: &[Expr]) -> Vec<V::Out> { es.iter().map(|e| e.accept(v)).collect() }

fn num(n: f64) -> Box<Expr> { Box::new(Expr::Num(n)) }
fn var(s: &str) -> Box<Expr> { Box::new(Expr::Var(s.into())) }

fn main() {
    let e1 = Expr::Bin(num(2.0), '*', Box::new(Expr::Bin(num(3.0), '+', var("x"))));
    let e2 = Expr::Let("x".into(), num(4.0), Box::new(e1.clone()));
    let e3 = Expr::Bin(num(1.0), '/', Box::new(Expr::Bin(var("y"), '-', var("y"))));
    let e4 = Expr::Let("y".into(), Box::new(Expr::Neg(num(2.5))), Box::new(e3.clone()));
    let e5 = Expr::Let("y".into(), num(7.0), Box::new(Expr::Bin(var("y"), '%', num(2.0))));
    let all = vec![e1, e2, e3, e4, e5];
    for s in run_all(&mut Printer, &all) { println!("{}", s); }
    for r in run_all(&mut Eval { env: vec![] }, &all) { println!("{:?}", r); }
    let mut c = Counter { nodes: 0, vars: vec![] };
    run_all(&mut c, &all);
    println!("{} {:?}", c.nodes, c.vars);
    println!("{:?}", run_all(&mut Depth, &all));
}
