// Hindley-Milner type inference (algorithm W with union-find substitution)
// for a tiny ML: let-polymorphism, recursive let, tuples, lists, if, lambdas.
// Includes an S-expression parser and pretty printing of generalized types.
use std::collections::{BTreeSet, HashMap};
use std::fmt;
use std::rc::Rc;

#[derive(Debug, Clone, PartialEq)]
enum Sexp {
    Atom(String),
    Int(i64),
    List(Vec<Sexp>),
}

fn tokenize(s: &str) -> Vec<String> {
    s.replace('(', " ( ").replace(')', " ) ").split_whitespace().map(String::from).collect()
}

fn parse_sexp(toks: &[String], pos: &mut usize) -> Result<Sexp, String> {
    let t = toks.get(*pos).ok_or("unexpected eof")?;
    *pos += 1;
    match t.as_str() {
        "(" => {
            let mut v = Vec::new();
            loop {
                match toks.get(*pos).map(String::as_str) {
                    Some(")") => {
                        *pos += 1;
                        return Ok(Sexp::List(v));
                    }
                    Some(_) => v.push(parse_sexp(toks, pos)?),
                    None => return Err("unclosed (".into()),
                }
            }
        }
        ")" => Err("unexpected )".into()),
        a => Ok(a.parse::<i64>().map(Sexp::Int).unwrap_or_else(|_| Sexp::Atom(a.to_string()))),
    }
}

#[derive(Debug, Clone)]
enum Expr {
    Int(i64),
    Bool(bool),
    Var(String),
    Lam(Vec<String>, Rc<Expr>),
    App(Rc<Expr>, Vec<Expr>),
    Let { name: String, rec: bool, val: Rc<Expr>, body: Rc<Expr> },
    If(Rc<Expr>, Rc<Expr>, Rc<Expr>),
    Tuple(Vec<Expr>),
}

fn to_expr(s: &Sexp) -> Result<Expr, String> {
    Ok(match s {
        Sexp::Int(n) => Expr::Int(*n),
        Sexp::Atom(a) if a == "true" || a == "false" => Expr::Bool(a == "true"),
        Sexp::Atom(a) => Expr::Var(a.clone()),
        Sexp::List(items) => match items.as_slice() {
            [] => Expr::Tuple(vec![]),
            [Sexp::Atom(kw), Sexp::List(params), body] if kw == "fn" => {
                let ps = params.iter().map(|p| match p {
                    Sexp::Atom(a) => Ok(a.clone()),
                    other => Err(format!("bad param {:?}", other)),
                });
                Expr::Lam(ps.collect::<Result<_, _>>()?, Rc::new(to_expr(body)?))
            }
            [Sexp::Atom(kw), Sexp::Atom(name), val, body] if kw == "let" || kw == "letrec" => Expr::Let {
                name: name.clone(),
                rec: kw == "letrec",
                val: Rc::new(to_expr(val)?),
                body: Rc::new(to_expr(body)?),
            },
            [Sexp::Atom(kw), c, t, e] if kw == "if" => Expr::If(Rc::new(to_expr(c)?), Rc::new(to_expr(t)?), Rc::new(to_expr(e)?)),
            [Sexp::Atom(kw), rest @ ..] if kw == "tuple" => Expr::Tuple(rest.iter().map(to_expr).collect::<Result<_, _>>()?),
            [f, args @ ..] => Expr::App(Rc::new(to_expr(f)?), args.iter().map(to_expr).collect::<Result<_, _>>()?),
        },
    })
}

#[derive(Debug, Clone, PartialEq)]
enum Type {
    Var(usize),
    Con(&'static str, Vec<Type>),
    Fun(Vec<Type>, Box<Type>),
}

#[derive(Debug, Clone)]
struct Scheme {
    vars: Vec<usize>,
    ty: Type,
}

#[derive(Debug)]
enum TypeError {
    Unbound(String),
    Mismatch(String, String),
    Occurs(usize, String),
    Arity(usize, usize),
}

impl fmt::Display for TypeError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            TypeError::Unbound(v) => write!(f, "unbound variable {}", v),
            TypeError::Mismatch(a, b) => write!(f, "cannot unify {} with {}", a, b),
            TypeError::Occurs(v, t) => write!(f, "infinite type: t{} occurs in {}", v, t),
            TypeError::Arity(a, b) => write!(f, "arity mismatch: {} vs {}", a, b),
        }
    }
}

struct Infer {
    subst: Vec<Option<Type>>,
    unifications: usize,
}

impl Infer {
    fn fresh(&mut self) -> Type {
        self.subst.push(None);
        Type::Var(self.subst.len() - 1)
    }
    fn prune(&mut self, t: &Type) -> Type {
        match t {
            Type::Var(v) => match self.subst[*v].clone() {
                Some(inner) => {
                    let p = self.prune(&inner);
                    self.subst[*v] = Some(p.clone()); // path compression
                    p
                }
                None => t.clone(),
            },
            _ => t.clone(),
        }
    }
    fn resolve(&mut self, t: &Type) -> Type {
        match self.prune(t) {
            Type::Con(n, args) => Type::Con(n, args.iter().map(|a| self.resolve(a)).collect()),
            Type::Fun(ps, r) => Type::Fun(ps.iter().map(|p| self.resolve(p)).collect(), Box::new(self.resolve(&r))),
            v => v,
        }
    }
    fn occurs(&mut self, v: usize, t: &Type) -> bool {
        match self.prune(t) {
            Type::Var(w) => v == w,
            Type::Con(_, args) => args.iter().any(|a| self.occurs(v, a)),
            Type::Fun(ps, r) => ps.iter().any(|p| self.occurs(v, p)) || self.occurs(v, &r),
        }
    }
    fn unify(&mut self, a: &Type, b: &Type) -> Result<(), TypeError> {
        self.unifications += 1;
        let (a, b) = (self.prune(a), self.prune(b));
        match (&a, &b) {
            (Type::Var(x), Type::Var(y)) if x == y => Ok(()),
            (Type::Var(x), t) | (t, Type::Var(x)) => {
                if self.occurs(*x, t) {
                    let shown = Pretty::new().show(&self.resolve(t));
                    return Err(TypeError::Occurs(*x, shown));
                }
                self.subst[*x] = Some(t.clone());
                Ok(())
            }
            (Type::Con(n1, a1), Type::Con(n2, a2)) if n1 == n2 && a1.len() == a2.len() => {
                for (x, y) in a1.iter().zip(a2) {
                    self.unify(x, y)?;
                }
                Ok(())
            }
            (Type::Fun(p1, r1), Type::Fun(p2, r2)) => {
                if p1.len() != p2.len() {
                    return Err(TypeError::Arity(p1.len(), p2.len()));
                }
                for (x, y) in p1.iter().zip(p2.iter()) {
                    self.unify(x, y)?;
                }
                self.unify(r1, r2)
            }
            _ => {
                let mut p = Pretty::new();
                let (sa, sb) = (self.resolve(&a), self.resolve(&b));
                Err(TypeError::Mismatch(p.show(&sa), p.show(&sb)))
            }
        }
    }
    fn free_vars(&mut self, t: &Type, out: &mut BTreeSet<usize>) {
        match self.prune(t) {
            Type::Var(v) => {
                out.insert(v);
            }
            Type::Con(_, args) => args.iter().for_each(|a| self.free_vars(a, out)),
            Type::Fun(ps, r) => {
                ps.iter().for_each(|p| self.free_vars(p, out));
                self.free_vars(&r, out);
            }
        }
    }
    fn instantiate(&mut self, s: &Scheme) -> Type {
        let map: HashMap<usize, Type> = s.vars.iter().map(|&v| (v, self.fresh())).collect();
        fn go(t: &Type, m: &HashMap<usize, Type>) -> Type {
            match t {
                Type::Var(v) => m.get(v).cloned().unwrap_or(Type::Var(*v)),
                Type::Con(n, a) => Type::Con(n, a.iter().map(|x| go(x, m)).collect()),
                Type::Fun(p, r) => Type::Fun(p.iter().map(|x| go(x, m)).collect(), Box::new(go(r, m))),
            }
        }
        let t = self.resolve(&s.ty);
        go(&t, &map)
    }
    fn generalize(&mut self, env: &Env, t: &Type) -> Scheme {
        let mut tv = BTreeSet::new();
        self.free_vars(t, &mut tv);
        let mut ev = BTreeSet::new();
        for s in env.iter_schemes() {
            let mut fv = BTreeSet::new();
            self.free_vars(&s.ty, &mut fv);
            for v in s.vars.iter() {
                fv.remove(v);
            }
            ev.extend(fv);
        }
        Scheme { vars: tv.difference(&ev).copied().collect(), ty: self.resolve(t) }
    }

    fn infer(&mut self, env: &Env, e: &Expr) -> Result<Type, TypeError> {
        match e {
            Expr::Int(_) => Ok(Type::Con("int", vec![])),
            Expr::Bool(_) => Ok(Type::Con("bool", vec![])),
            Expr::Var(v) => {
                let s = env.lookup(v).ok_or_else(|| TypeError::Unbound(v.clone()))?;
                Ok(self.instantiate(&s))
            }
            Expr::Lam(ps, body) => {
                let pts: Vec<Type> = ps.iter().map(|_| self.fresh()).collect();
                let mut inner = env.clone();
                for (p, t) in ps.iter().zip(&pts) {
                    inner = inner.extend(p, Scheme { vars: vec![], ty: t.clone() });
                }
                let rt = self.infer(&inner, body)?;
                Ok(Type::Fun(pts, Box::new(rt)))
            }
            Expr::App(f, args) => {
                let ft = self.infer(env, f)?;
                let ats = args.iter().map(|a| self.infer(env, a)).collect::<Result<Vec<_>, _>>()?;
                let rt = self.fresh();
                self.unify(&ft, &Type::Fun(ats, Box::new(rt.clone())))?;
                Ok(rt)
            }
            Expr::Let { name, rec, val, body } => {
                let vt = if *rec {
                    let tv = self.fresh();
                    let inner = env.extend(name, Scheme { vars: vec![], ty: tv.clone() });
                    let t = self.infer(&inner, val)?;
                    self.unify(&tv, &t)?;
                    t
                } else {
                    self.infer(env, val)?
                };
                let s = self.generalize(env, &vt);
                self.infer(&env.extend(name, s), body)
            }
            Expr::If(c, t, f) => {
                let ct = self.infer(env, c)?;
                self.unify(&ct, &Type::Con("bool", vec![]))?;
                let tt = self.infer(env, t)?;
                let ft = self.infer(env, f)?;
                self.unify(&tt, &ft)?;
                Ok(tt)
            }
            Expr::Tuple(es) => Ok(Type::Con("tuple", es.iter().map(|x| self.infer(env, x)).collect::<Result<_, _>>()?)),
        }
    }
}

// persistent environment as a linked list of Rc frames
#[derive(Clone)]
struct Env(Option<Rc<(String, Scheme, Env)>>);

impl Env {
    fn extend(&self, n: &str, s: Scheme) -> Env {
        Env(Some(Rc::new((n.to_string(), s, self.clone()))))
    }
    fn lookup(&self, n: &str) -> Option<Scheme> {
        let mut cur = self;
        while let Some(frame) = &cur.0 {
            if frame.0 == n {
                return Some(frame.1.clone());
            }
            cur = &frame.2;
        }
        None
    }
    fn iter_schemes(&self) -> impl Iterator<Item = &Scheme> + '_ {
        let mut cur = self;
        std::iter::from_fn(move || {
            let frame = cur.0.as_ref()?;
            cur = &frame.2;
            Some(&frame.1)
        })
    }
}

struct Pretty {
    names: HashMap<usize, String>,
}

impl Pretty {
    fn new() -> Self {
        Pretty { names: HashMap::new() }
    }
    fn name(&mut self, v: usize) -> String {
        let n = self.names.len();
        self.names
            .entry(v)
            .or_insert_with(|| {
                let letter = (b'a' + (n % 26) as u8) as char;
                if n >= 26 {
                    format!("'{}{}", letter, n / 26)
                } else {
                    format!("'{}", letter)
                }
            })
            .clone()
    }
    fn show(&mut self, t: &Type) -> String {
        match t {
            Type::Var(v) => self.name(*v),
            Type::Con(n, a) if a.is_empty() => n.to_string(),
            Type::Con("tuple", a) => format!("({})", a.iter().map(|x| self.show(x)).collect::<Vec<_>>().join(" * ")),
            Type::Con(n, a) => format!("{} {}", a.iter().map(|x| self.show(x)).collect::<Vec<_>>().join(" "), n),
            Type::Fun(ps, r) => {
                let p: Vec<String> = ps.iter().map(|x| match x {
                    Type::Fun(..) => format!("({})", self.show(x)),
                    _ => self.show(x),
                }).collect();
                let ps = if p.is_empty() { "unit".to_string() } else { p.join(", ") };
                format!("{} -> {}", ps, self.show(r))
            }
        }
    }
}

fn prelude(inf: &mut Infer) -> Env {
    let int = || Type::Con("int", vec![]);
    let boolt = || Type::Con("bool", vec![]);
    let list = |t: Type| Type::Con("list", vec![t]);
    let mut env = Env(None);
    let fun = |ps: Vec<Type>, r: Type| Type::Fun(ps, Box::new(r));
    for op in ["+", "-", "*"] {
        env = env.extend(op, Scheme { vars: vec![], ty: fun(vec![int(), int()], int()) });
    }
    env = env.extend("<", Scheme { vars: vec![], ty: fun(vec![int(), int()], boolt()) });
    let a = inf.fresh();
    let Type::Var(av) = a else { unreachable!() };
    env = env.extend("=", Scheme { vars: vec![av], ty: fun(vec![a.clone(), a.clone()], boolt()) });
    env = env.extend("nil", Scheme { vars: vec![av], ty: list(a.clone()) });
    env = env.extend("cons", Scheme { vars: vec![av], ty: fun(vec![a.clone(), list(a.clone())], list(a.clone())) });
    env = env.extend("head", Scheme { vars: vec![av], ty: fun(vec![list(a.clone())], a.clone()) });
    env = env.extend("tail", Scheme { vars: vec![av], ty: fun(vec![list(a.clone())], list(a.clone())) });
    env = env.extend("null", Scheme { vars: vec![av], ty: fun(vec![list(a.clone())], boolt()) });
    let b = inf.fresh();
    let Type::Var(bv) = b else { unreachable!() };
    env = env.extend("fst", Scheme { vars: vec![av, bv], ty: fun(vec![Type::Con("tuple", vec![a.clone(), b.clone()])], a.clone()) });
    env.extend("snd", Scheme { vars: vec![av, bv], ty: fun(vec![Type::Con("tuple", vec![a, b.clone()])], b) })
}

fn main() {
    let programs = [
        "42",
        "(fn (x) x)",
        "(fn (f g x) (f (g x)))",
        "(let id (fn (x) x) (tuple (id 1) (id true)))",
        "(letrec len (fn (l) (if (null l) 0 (+ 1 (len (tail l))))) len)",
        "(letrec map (fn (f l) (if (null l) nil (cons (f (head l)) (map f (tail l))))) map)",
        "(letrec fold (fn (f acc l) (if (null l) acc (fold f (f acc (head l)) (tail l)))) fold)",
        "(let compose (fn (f g) (fn (x) (f (g x)))) (compose (fn (x) (< x 3)) (fn (y) (* y 2))))",
        "(fn (p) (tuple (snd p) (fst p)))",
        "(fn (x) (x x))",
        "(+ 1 true)",
        "(fn (f) (tuple (f 1) (f true)))",
        "(undefined 3)",
        "((fn (x y) x) 1)",
        "(let k (fn (x) (fn (y) x)) (tuple (k 1) ((k true) 5)))",
        "(letrec fact (fn (n) (if (< n 2) 1 (* n (fact (- n 1))))) (fact 10))",
        "(fn (a b c d e) (tuple e d c b a (= a b)))",
        "(cons (tuple 1 nil) nil)",
        "(let pair (fn (x) (tuple x x)) (pair (pair (pair 1))))",
        "(if true 1",
    ];
    let mut total_unify = 0;
    for src in programs {
        let toks = tokenize(src);
        let mut pos = 0;
        let parsed = parse_sexp(&toks, &mut pos).and_then(|s| to_expr(&s));
        let e = match parsed {
            Ok(e) => e,
            Err(msg) => {
                println!("{:<70} parse error: {}", src, msg);
                continue;
            }
        };
        let mut inf = Infer { subst: Vec::new(), unifications: 0 };
        let env = prelude(&mut inf);
        let r = inf.infer(&env, &e);
        total_unify += inf.unifications;
        match r {
            Ok(t) => {
                let t = inf.resolve(&t);
                let shown = Pretty::new().show(&t);
                println!("{:<70} : {}", src, shown);
            }
            Err(err) => println!("{:<70} ! {}", src, err),
        }
    }
    println!("total unifications: {}", total_unify);
}
