use std::fmt;
use std::rc::Rc;
use std::collections::HashMap;

#[derive(Debug, Clone)]
enum Expr { Num(i64), Bool(bool), Var(String), Lam(String, Rc<Expr>), App(Rc<Expr>, Rc<Expr>), Let(String, Rc<Expr>, Rc<Expr>), If(Rc<Expr>, Rc<Expr>, Rc<Expr>), Prim(&'static str, Vec<Expr>), LetRec(String, String, Rc<Expr>, Rc<Expr>) }

#[derive(Clone)]
enum Val { Num(i64), Bool(bool), Closure(String, Rc<Expr>, Env), Rec(String, String, Rc<Expr>, Env) }

#[derive(Clone, Default)]
struct Env(Option<Rc<(String, Val, Env)>>);
impl Env {
    fn extend(&self, k: &str, v: Val) -> Env { Env(Some(Rc::new((k.to_string(), v, self.clone())))) }
    fn lookup(&self, k: &str) -> Option<Val> {
        let mut cur = self;
        while let Some(node) = &cur.0 {
            if node.0 == k {
                return Some(match &node.1 { Val::Rec(f, x, b, e) => { let _ = (f, x, b, e); node.1.clone() } v => v.clone() });
            }
            cur = &node.2;
        }
        None
    }
    fn depth(&self) -> usize { let mut n = 0; let mut c = self; while let Some(node) = &c.0 { n += 1; c = &node.2; } n }
}

impl fmt::Display for Val {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self { Val::Num(n) => write!(f, "{}", n), Val::Bool(b) => write!(f, "{}", b), Val::Closure(x, _, e) => write!(f, "<fn {} env{}>", x, e.depth()), Val::Rec(n, ..) => write!(f, "<rec {}>", n) }
    }
}

fn eval(e: &Expr, env: &Env, steps: &mut u64) -> Result<Val, String> {
    *steps += 1;
    if *steps > 3_000 { return Err("step limit".into()); }
    match e {
        Expr::Num(n) => Ok(Val::Num(*n)),
        Expr::Bool(b) => Ok(Val::Bool(*b)),
        Expr::Var(x) => env.lookup(x).ok_or_else(|| format!("unbound {}", x)),
        Expr::Lam(x, b) => Ok(Val::Closure(x.clone(), b.clone(), env.clone())),
        Expr::Let(x, v, b) => { let vv = eval(v, env, steps)?; eval(b, &env.extend(x, vv), steps) }
        Expr::LetRec(f, x, body, rest) => { let rv = Val::Rec(f.clone(), x.clone(), body.clone(), env.clone()); eval(rest, &env.extend(f, rv), steps) }
        Expr::If(c, t, el) => match eval(c, env, steps)? { Val::Bool(true) => eval(t, env, steps), Val::Bool(false) => eval(el, env, steps), v => Err(format!("if on non-bool {}", v)) },
        Expr::App(fe, ae) => {
            let fv = eval(fe, env, steps)?;
            let av = eval(ae, env, steps)?;
            match fv {
                Val::Closure(x, b, cenv) => eval(&b, &cenv.extend(&x, av), steps),
                Val::Rec(ref f, ref x, ref b, ref renv) => { let e2 = renv.extend(f, fv.clone()).extend(x, av); eval(b, &e2, steps) }
                v => Err(format!("apply non-function {}", v)),
            }
        }
        Expr::Prim(op, args) => {
            let vs: Vec<Val> = args.iter().map(|a| eval(a, env, steps)).collect::<Result<_, _>>()?;
            match (*op, vs.as_slice()) {
                ("+", [Val::Num(a), Val::Num(b)]) => Ok(Val::Num(a + b)),
                ("-", [Val::Num(a), Val::Num(b)]) => Ok(Val::Num(a - b)),
                ("*", [Val::Num(a), Val::Num(b)]) => Ok(Val::Num(a.wrapping_mul(*b))),
                ("<", [Val::Num(a), Val::Num(b)]) => Ok(Val::Bool(a < b)),
                ("=", [Val::Num(a), Val::Num(b)]) => Ok(Val::Bool(a == b)),
                ("not", [Val::Bool(b)]) => Ok(Val::Bool(!b)),
                (op, vs) => Err(format!("bad prim {} with {} args", op, vs.len())),
            }
        }
    }
}

fn num(n: i64) -> Rc<Expr> { Rc::new(Expr::Num(n)) }
fn var(s: &str) -> Rc<Expr> { Rc::new(Expr::Var(s.into())) }
fn lam(x: &str, b: Rc<Expr>) -> Rc<Expr> { Rc::new(Expr::Lam(x.into(), b)) }
fn app(f: Rc<Expr>, a: Rc<Expr>) -> Rc<Expr> { Rc::new(Expr::App(f, a)) }
fn prim(op: &'static str, args: Vec<Rc<Expr>>) -> Rc<Expr> { Rc::new(Expr::Prim(op, args.into_iter().map(|a| (*a).clone()).collect())) }

fn main() {
    let mut programs: Vec<(&str, Rc<Expr>)> = vec![];
    programs.push(("id 5", app(lam("x", var("x")), num(5))));
    programs.push(("const", app(app(lam("x", lam("y", var("x"))), num(1)), num(2))));
    let fact_body = Rc::new(Expr::If(prim("<", vec![var("n"), num(2)]), num(1), prim("*", vec![var("n"), app(var("fact"), prim("-", vec![var("n"), num(1)]))])));
    programs.push(("fact 20", Rc::new(Expr::LetRec("fact".into(), "n".into(), fact_body.clone(), app(var("fact"), num(20))))));
    let fib_body = Rc::new(Expr::If(prim("<", vec![var("n"), num(2)]), var("n"), prim("+", vec![app(var("fib"), prim("-", vec![var("n"), num(1)])), app(var("fib"), prim("-", vec![var("n"), num(2)]))])));
    programs.push(("fib 12", Rc::new(Expr::LetRec("fib".into(), "n".into(), fib_body, app(var("fib"), num(12))))));
    let compose = lam("f", lam("g", lam("x", app(var("f"), app(var("g"), var("x"))))));
    let inc = lam("x", prim("+", vec![var("x"), num(1)]));
    let dbl = lam("x", prim("*", vec![var("x"), num(2)]));
    programs.push(("compose", app(app(app(compose.clone(), inc.clone()), dbl.clone()), num(10))));
    let church = |n: usize| { let mut b = var("x"); for _ in 0..n { b = app(var("f"), b); } lam("f", lam("x", b)) };
    programs.push(("church 7", app(app(church(7), inc.clone()), num(0))));
    let mult = lam("m", lam("n", lam("f", app(var("m"), app(var("n"), var("f"))))));
    programs.push(("church 6*7", app(app(app(app(mult, church(6)), church(7)), inc.clone()), num(0))));
    programs.push(("shadow", Rc::new(Expr::Let("x".into(), num(1), Rc::new(Expr::Let("f".into(), lam("y", prim("+", vec![var("x"), var("y")])), Rc::new(Expr::Let("x".into(), num(100), app(var("f"), var("x"))))))))));
    programs.push(("closure value", app(compose, inc)));
    programs.push(("unbound", app(lam("x", var("y")), num(1))));
    programs.push(("bad if", Rc::new(Expr::If(num(1), num(2), num(3)))));
    programs.push(("apply num", app(num(3), num(4))));
    programs.push(("bad prim", prim("not", vec![num(1)])));
    let omega = lam("x", app(var("x"), var("x")));
    programs.push(("omega", app(omega.clone(), omega)));
    let mut stats: HashMap<&str, u64> = HashMap::new();
    for (name, p) in &programs {
        let mut steps = 0;
        let r = eval(p, &Env::default(), &mut steps);
        stats.insert(name, steps);
        match r { Ok(v) => println!("{:<14} => {:<20} ({} steps)", name, v.to_string(), steps), Err(e) => println!("{:<14} !! {} ({} steps)", name, e, steps) }
    }
    let mut s: Vec<_> = stats.into_iter().collect();
    s.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(b.0)));
    println!("{:?}", &s[..3]);
}
