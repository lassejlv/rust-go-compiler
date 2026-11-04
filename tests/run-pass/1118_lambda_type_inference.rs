// Hindley-Milner type inference for a small lambda calculus with let-
// polymorphism, integers, booleans, pairs, lists, fix and if; plus a
// call-by-value evaluator. Uses union-find substitution in a Vec.
use std::collections::{BTreeSet, HashMap};
use std::fmt;
use std::rc::Rc;

#[derive(Debug, Clone)]
enum Term {
    Int(i64),
    Bool(bool),
    Var(String),
    Lam(String, Rc<Term>),
    App(Rc<Term>, Rc<Term>),
    Let(String, Rc<Term>, Rc<Term>),
    If(Rc<Term>, Rc<Term>, Rc<Term>),
    Fix(Rc<Term>),
    Pair(Rc<Term>, Rc<Term>),
    Nil,
    Prim(&'static str),
}

#[derive(Debug, Clone, PartialEq)]
enum Ty {
    Var(usize),
    Con(&'static str),
    Fun(Box<Ty>, Box<Ty>),
    Pair(Box<Ty>, Box<Ty>),
    List(Box<Ty>),
}

#[derive(Debug, Clone)]
struct Scheme { vars: Vec<usize>, ty: Ty }

#[derive(Debug)]
enum TypeError { Mismatch(String, String), Occurs(usize, String), Unbound(String) }

impl fmt::Display for TypeError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            TypeError::Mismatch(a, b) => write!(f, "cannot unify {} with {}", a, b),
            TypeError::Occurs(v, t) => write!(f, "infinite type: t{} occurs in {}", v, t),
            TypeError::Unbound(n) => write!(f, "unbound variable {}", n),
        }
    }
}

struct Infer { subst: Vec<Option<Ty>>, steps: usize }

impl Infer {
    fn fresh(&mut self) -> Ty {
        self.subst.push(None);
        Ty::Var(self.subst.len() - 1)
    }
    fn resolve(&self, t: &Ty) -> Ty {
        match t {
            Ty::Var(v) => match &self.subst[*v] {
                Some(t2) => self.resolve(t2),
                None => t.clone(),
            },
            Ty::Con(_) => t.clone(),
            Ty::Fun(a, b) => Ty::Fun(Box::new(self.resolve(a)), Box::new(self.resolve(b))),
            Ty::Pair(a, b) => Ty::Pair(Box::new(self.resolve(a)), Box::new(self.resolve(b))),
            Ty::List(a) => Ty::List(Box::new(self.resolve(a))),
        }
    }
    fn occurs(&self, v: usize, t: &Ty) -> bool {
        match self.resolve(t) {
            Ty::Var(w) => v == w,
            Ty::Con(_) => false,
            Ty::Fun(a, b) | Ty::Pair(a, b) => self.occurs(v, &a) || self.occurs(v, &b),
            Ty::List(a) => self.occurs(v, &a),
        }
    }
    fn unify(&mut self, a: &Ty, b: &Ty) -> Result<(), TypeError> {
        self.steps += 1;
        let (a, b) = (self.resolve(a), self.resolve(b));
        match (&a, &b) {
            (Ty::Var(x), Ty::Var(y)) if x == y => Ok(()),
            (Ty::Var(x), t) | (t, Ty::Var(x)) => {
                if self.occurs(*x, t) {
                    return Err(TypeError::Occurs(*x, pretty(t, &mut HashMap::new())));
                }
                self.subst[*x] = Some(t.clone());
                Ok(())
            }
            (Ty::Con(x), Ty::Con(y)) if x == y => Ok(()),
            (Ty::Fun(a1, b1), Ty::Fun(a2, b2)) | (Ty::Pair(a1, b1), Ty::Pair(a2, b2)) => {
                self.unify(a1, a2)?;
                self.unify(b1, b2)
            }
            (Ty::List(x), Ty::List(y)) => self.unify(x, y),
            _ => {
                let mut names = HashMap::new();
                Err(TypeError::Mismatch(pretty(&a, &mut names), pretty(&b, &mut names)))
            }
        }
    }
    fn free_vars(&self, t: &Ty, out: &mut BTreeSet<usize>) {
        match self.resolve(t) {
            Ty::Var(v) => { out.insert(v); }
            Ty::Con(_) => {}
            Ty::Fun(a, b) | Ty::Pair(a, b) => { self.free_vars(&a, out); self.free_vars(&b, out); }
            Ty::List(a) => self.free_vars(&a, out),
        }
    }
    fn generalize(&self, env: &Env, t: &Ty) -> Scheme {
        let mut ft = BTreeSet::new();
        self.free_vars(t, &mut ft);
        let mut fe = BTreeSet::new();
        for s in env.values() {
            let mut f = BTreeSet::new();
            self.free_vars(&s.ty, &mut f);
            for v in &s.vars { f.remove(v); }
            fe.extend(f);
        }
        Scheme { vars: ft.difference(&fe).copied().collect(), ty: self.resolve(t) }
    }
    fn instantiate(&mut self, s: &Scheme) -> Ty {
        let map: HashMap<usize, Ty> = s.vars.iter().map(|&v| (v, self.fresh())).collect();
        fn go(t: &Ty, m: &HashMap<usize, Ty>) -> Ty {
            match t {
                Ty::Var(v) => m.get(v).cloned().unwrap_or(Ty::Var(*v)),
                Ty::Con(_) => t.clone(),
                Ty::Fun(a, b) => Ty::Fun(Box::new(go(a, m)), Box::new(go(b, m))),
                Ty::Pair(a, b) => Ty::Pair(Box::new(go(a, m)), Box::new(go(b, m))),
                Ty::List(a) => Ty::List(Box::new(go(a, m))),
            }
        }
        go(&self.resolve(&s.ty), &map)
    }

    fn prim_type(&mut self, p: &str) -> Ty {
        let int = || Ty::Con("Int");
        let boolean = || Ty::Con("Bool");
        let f = |a: Ty, b: Ty| Ty::Fun(Box::new(a), Box::new(b));
        match p {
            "+" | "-" | "*" => f(int(), f(int(), int())),
            "==" | "<" => f(int(), f(int(), boolean())),
            "fst" => { let a = self.fresh(); let b = self.fresh(); f(Ty::Pair(Box::new(a.clone()), Box::new(b)), a) }
            "snd" => { let a = self.fresh(); let b = self.fresh(); f(Ty::Pair(Box::new(a), Box::new(b.clone())), b) }
            "cons" => { let a = self.fresh(); f(a.clone(), f(Ty::List(Box::new(a.clone())), Ty::List(Box::new(a)))) }
            "head" => { let a = self.fresh(); f(Ty::List(Box::new(a.clone())), a) }
            "tail" => { let a = self.fresh(); f(Ty::List(Box::new(a.clone())), Ty::List(Box::new(a))) }
            "null" => { let a = self.fresh(); f(Ty::List(Box::new(a)), boolean()) }
            _ => unreachable!("prim {}", p),
        }
    }

    fn infer(&mut self, env: &Env, t: &Term) -> Result<Ty, TypeError> {
        Ok(match t {
            Term::Int(_) => Ty::Con("Int"),
            Term::Bool(_) => Ty::Con("Bool"),
            Term::Nil => Ty::List(Box::new(self.fresh())),
            Term::Prim(p) => self.prim_type(p),
            Term::Var(n) => {
                let s = env.get(n).ok_or_else(|| TypeError::Unbound(n.clone()))?.clone();
                self.instantiate(&s)
            }
            Term::Lam(x, body) => {
                let tv = self.fresh();
                let mut env2 = env.clone();
                env2.insert(x.clone(), Scheme { vars: vec![], ty: tv.clone() });
                let tb = self.infer(&env2, body)?;
                Ty::Fun(Box::new(tv), Box::new(tb))
            }
            Term::App(f, a) => {
                let tf = self.infer(env, f)?;
                let ta = self.infer(env, a)?;
                let tr = self.fresh();
                self.unify(&tf, &Ty::Fun(Box::new(ta), Box::new(tr.clone())))?;
                tr
            }
            Term::Let(x, e, body) => {
                let te = self.infer(env, e)?;
                let s = self.generalize(env, &te);
                let mut env2 = env.clone();
                env2.insert(x.clone(), s);
                self.infer(&env2, body)?
            }
            Term::If(c, a, b) => {
                let tc = self.infer(env, c)?;
                self.unify(&tc, &Ty::Con("Bool"))?;
                let ta = self.infer(env, a)?;
                let tb = self.infer(env, b)?;
                self.unify(&ta, &tb)?;
                ta
            }
            Term::Fix(f) => {
                let tf = self.infer(env, f)?;
                let a = self.fresh();
                self.unify(&tf, &Ty::Fun(Box::new(a.clone()), Box::new(a.clone())))?;
                a
            }
            Term::Pair(a, b) => Ty::Pair(Box::new(self.infer(env, a)?), Box::new(self.infer(env, b)?)),
        })
    }
}

type Env = HashMap<String, Scheme>;

fn pretty(t: &Ty, names: &mut HashMap<usize, String>) -> String {
    match t {
        Ty::Var(v) => {
            let n = names.len();
            names.entry(*v).or_insert_with(|| {
                let c = (b'a' + (n % 26) as u8) as char;
                if n >= 26 { format!("{}{}", c, n / 26) } else { c.to_string() }
            }).clone()
        }
        Ty::Con(c) => c.to_string(),
        Ty::Fun(a, b) => {
            let l = pretty(a, names);
            let l = if matches!(**a, Ty::Fun(..)) { format!("({})", l) } else { l };
            format!("{} -> {}", l, pretty(b, names))
        }
        Ty::Pair(a, b) => format!("({}, {})", pretty(a, names), pretty(b, names)),
        Ty::List(a) => format!("[{}]", pretty(a, names)),
    }
}

// ---------- parser ----------
// syntax: \x. e | let x = e in e | if e then e else e | fix e | e e | (e, e) | atoms
struct P { toks: Vec<String>, i: usize }

impl P {
    fn new(s: &str) -> P {
        let mut toks = Vec::new();
        let cs: Vec<char> = s.chars().collect();
        let mut i = 0;
        while i < cs.len() {
            let c = cs[i];
            if c.is_whitespace() { i += 1; continue; }
            if c.is_alphanumeric() || c == '_' {
                let st = i;
                while i < cs.len() && (cs[i].is_alphanumeric() || cs[i] == '_' || cs[i] == '\'') { i += 1; }
                toks.push(cs[st..i].iter().collect());
                continue;
            }
            if c == '=' && cs.get(i + 1) == Some(&'=') { toks.push("==".into()); i += 2; continue; }
            toks.push(c.to_string());
            i += 1;
        }
        P { toks, i: 0 }
    }
    fn peek(&self) -> Option<&str> { self.toks.get(self.i).map(|s| s.as_str()) }
    fn eat(&mut self, s: &str) -> Result<(), String> {
        if self.peek() == Some(s) { self.i += 1; Ok(()) } else { Err(format!("expected {:?} at token {} (found {:?})", s, self.i, self.peek())) }
    }
    fn ident(&mut self) -> Result<String, String> {
        let t = self.peek().ok_or("unexpected end")?.to_string();
        if t.chars().next().map_or(false, |c| c.is_alphabetic()) { self.i += 1; Ok(t) } else { Err(format!("expected identifier, found {:?}", t)) }
    }
    fn expr(&mut self) -> Result<Term, String> {
        match self.peek() {
            Some("\\") => {
                self.i += 1;
                let mut params = vec![self.ident()?];
                while self.peek() != Some(".") { params.push(self.ident()?); }
                self.eat(".")?;
                let body = self.expr()?;
                Ok(params.into_iter().rev().fold(body, |b, p| Term::Lam(p, Rc::new(b))))
            }
            Some("let") => {
                self.i += 1;
                let x = self.ident()?;
                let mut params = Vec::new();
                while self.peek() != Some("=") { params.push(self.ident()?); }
                self.eat("=")?;
                let e = self.expr()?;
                let e = params.into_iter().rev().fold(e, |b, p| Term::Lam(p, Rc::new(b)));
                self.eat("in")?;
                let body = self.expr()?;
                Ok(Term::Let(x, Rc::new(e), Rc::new(body)))
            }
            Some("if") => {
                self.i += 1;
                let c = self.expr()?;
                self.eat("then")?;
                let a = self.expr()?;
                self.eat("else")?;
                let b = self.expr()?;
                Ok(Term::If(Rc::new(c), Rc::new(a), Rc::new(b)))
            }
            _ => self.binop(),
        }
    }
    fn binop(&mut self) -> Result<Term, String> {
        let mut lhs = self.app()?;
        while let Some(op) = self.peek() {
            let p: &'static str = match op { "+" => "+", "-" => "-", "*" => "*", "<" => "<", "==" => "==", _ => break };
            self.i += 1;
            let rhs = self.app()?;
            lhs = Term::App(Rc::new(Term::App(Rc::new(Term::Prim(p)), Rc::new(lhs))), Rc::new(rhs));
        }
        Ok(lhs)
    }
    fn app(&mut self) -> Result<Term, String> {
        let mut f = self.atom()?;
        while let Some(t) = self.peek() {
            if t == "(" || t == "[" || t.chars().next().unwrap().is_alphanumeric() && !["then", "else", "in", "let", "if"].contains(&t) {
                let a = self.atom()?;
                f = Term::App(Rc::new(f), Rc::new(a));
            } else {
                break;
            }
        }
        Ok(f)
    }
    fn atom(&mut self) -> Result<Term, String> {
        let t = self.peek().ok_or("unexpected end of input")?.to_string();
        self.i += 1;
        Ok(match t.as_str() {
            "(" => {
                let e = self.expr()?;
                if self.peek() == Some(",") {
                    self.i += 1;
                    let b = self.expr()?;
                    self.eat(")")?;
                    Term::Pair(Rc::new(e), Rc::new(b))
                } else {
                    self.eat(")")?;
                    e
                }
            }
            "[" => { self.eat("]")?; Term::Nil }
            "true" => Term::Bool(true),
            "false" => Term::Bool(false),
            "fix" => Term::Fix(Rc::new(self.atom()?)),
            "fst" | "snd" | "cons" | "head" | "tail" | "null" => Term::Prim(match t.as_str() { "fst" => "fst", "snd" => "snd", "cons" => "cons", "head" => "head", "tail" => "tail", _ => "null" }),
            s if s.chars().all(|c| c.is_ascii_digit()) => Term::Int(s.parse().map_err(|e| format!("{}", e))?),
            s if s.chars().next().unwrap().is_alphabetic() => Term::Var(s.to_string()),
            s => return Err(format!("unexpected token {:?}", s)),
        })
    }
}

// ---------- evaluator ----------
#[derive(Clone)]
enum Val {
    Int(i64),
    Bool(bool),
    Pair(Rc<Val>, Rc<Val>),
    List(Rc<Vec<Val>>),
    Clo(String, Rc<Term>, Rc<VEnv>),
    Prim(&'static str, Vec<Val>),
    Fix(Rc<Term>, Rc<VEnv>),
}

#[derive(Default)]
struct VEnv { vars: Vec<(String, Val)> }

impl fmt::Display for Val {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Val::Int(n) => write!(f, "{}", n),
            Val::Bool(b) => write!(f, "{}", b),
            Val::Pair(a, b) => write!(f, "({}, {})", a, b),
            Val::List(v) => {
                write!(f, "[")?;
                for (i, x) in v.iter().enumerate() { if i > 0 { write!(f, ", ")?; } write!(f, "{}", x)?; }
                write!(f, "]")
            }
            Val::Clo(..) | Val::Fix(..) => write!(f, "<closure>"),
            Val::Prim(p, args) => write!(f, "<prim {} /{}>", p, args.len()),
        }
    }
}

fn lookup(env: &VEnv, n: &str) -> Val {
    env.vars.iter().rev().find(|(k, _)| k == n).map(|(_, v)| v.clone()).expect("typechecked")
}

fn extend(env: &Rc<VEnv>, k: &str, v: Val) -> Rc<VEnv> {
    let mut vars = env.vars.clone();
    vars.push((k.to_string(), v));
    Rc::new(VEnv { vars })
}

fn eval(t: &Term, env: &Rc<VEnv>, fuel: &mut u64) -> Result<Val, String> {
    *fuel = fuel.checked_sub(1).ok_or("out of fuel")?;
    Ok(match t {
        Term::Int(n) => Val::Int(*n),
        Term::Bool(b) => Val::Bool(*b),
        Term::Nil => Val::List(Rc::new(vec![])),
        Term::Prim(p) => Val::Prim(p, vec![]),
        Term::Var(n) => lookup(env, n),
        Term::Lam(x, b) => Val::Clo(x.clone(), b.clone(), env.clone()),
        Term::App(f, a) => {
            let fv = eval(f, env, fuel)?;
            let av = eval(a, env, fuel)?;
            apply(fv, av, fuel)?
        }
        Term::Let(x, e, b) => {
            let v = eval(e, env, fuel)?;
            eval(b, &extend(env, x, v), fuel)?
        }
        Term::If(c, a, b) => match eval(c, env, fuel)? {
            Val::Bool(true) => eval(a, env, fuel)?,
            _ => eval(b, env, fuel)?,
        },
        Term::Fix(f) => Val::Fix(f.clone(), env.clone()),
        Term::Pair(a, b) => Val::Pair(Rc::new(eval(a, env, fuel)?), Rc::new(eval(b, env, fuel)?)),
    })
}

fn apply(f: Val, a: Val, fuel: &mut u64) -> Result<Val, String> {
    match f {
        Val::Clo(x, body, cenv) => eval(&body, &extend(&cenv, &x, a), fuel),
        Val::Fix(ft, fenv) => {
            let inner = eval(&ft, &fenv, fuel)?;
            let unrolled = apply(inner, Val::Fix(ft.clone(), fenv.clone()), fuel)?;
            apply(unrolled, a, fuel)
        }
        Val::Prim(p, mut args) => {
            args.push(a);
            let arity = if matches!(p, "+" | "-" | "*" | "==" | "<" | "cons") { 2 } else { 1 };
            if args.len() < arity { return Ok(Val::Prim(p, args)); }
            Ok(match (p, args.as_slice()) {
                ("+", [Val::Int(x), Val::Int(y)]) => Val::Int(x.wrapping_add(*y)),
                ("-", [Val::Int(x), Val::Int(y)]) => Val::Int(x.wrapping_sub(*y)),
                ("*", [Val::Int(x), Val::Int(y)]) => Val::Int(x.wrapping_mul(*y)),
                ("==", [Val::Int(x), Val::Int(y)]) => Val::Bool(x == y),
                ("<", [Val::Int(x), Val::Int(y)]) => Val::Bool(x < y),
                ("fst", [Val::Pair(a, _)]) => (**a).clone(),
                ("snd", [Val::Pair(_, b)]) => (**b).clone(),
                ("cons", [x, Val::List(xs)]) => { let mut v = vec![x.clone()]; v.extend(xs.iter().cloned()); Val::List(Rc::new(v)) }
                ("head", [Val::List(xs)]) => xs.first().cloned().ok_or("head of empty list")?,
                ("tail", [Val::List(xs)]) => if xs.is_empty() { return Err("tail of empty list".into()) } else { Val::List(Rc::new(xs[1..].to_vec())) },
                ("null", [Val::List(xs)]) => Val::Bool(xs.is_empty()),
                _ => return Err(format!("bad primitive application {}", p)),
            })
        }
        _ => Err("applying a non-function".into()),
    }
}

fn main() {
    let progs = [
        "\\x. x",
        "\\f g x. f (g x)",
        "\\x y. x",
        "let id = \\x. x in (id 1, id true)",
        "let twice f x = f (f x) in twice (\\n. n * 2) 5",
        "\\p. (snd p, fst p)",
        "fix (\\fact n. if n < 2 then 1 else n * fact (n - 1)) 10",
        "let map = fix (\\map f xs. if null xs then [] else cons (f (head xs)) (map f (tail xs))) in map (\\x. (x, x == 3)) (cons 1 (cons 2 (cons 3 [])))",
        "let foldr = fix (\\foldr f z xs. if null xs then z else f (head xs) (foldr f z (tail xs))) in foldr (\\a b. a + b) 0 (cons 10 (cons 20 (cons 12 [])))",
        "let compose f g x = f (g x) in let inc x = x + 1 in compose inc inc 40",
        "\\x. x x",
        "1 + true",
        "if 1 then 2 else 3",
        "\\f. (f 1, f true)",
        "let f = \\x. y in f",
        "let fib = fix (\\fib n. if n < 2 then n else fib (n - 1) + fib (n - 2)) in fib 20",
        "let s f g x = f x (g x) in let k x y = x in s k k",
        "head []",
        "(\\x. ) 1",
        "let pair = \\a b. (a, b) in pair [] (pair 1 [])",
    ];
    for src in progs.iter() {
        println!("> {}", src);
        let term = match P::new(src).expr() {
            Ok(t) => t,
            Err(e) => { println!("  parse error: {}", e); continue; }
        };
        let mut inf = Infer { subst: vec![], steps: 0 };
        match inf.infer(&HashMap::new(), &term) {
            Err(e) => println!("  type error: {} (after {} unifications)", e, inf.steps),
            Ok(ty) => {
                let r = inf.resolve(&ty);
                println!("  : {}   [{} vars, {} unifications]", pretty(&r, &mut HashMap::new()), inf.subst.len(), inf.steps);
                let mut fuel = 200_000;
                match eval(&term, &Rc::new(VEnv::default()), &mut fuel) {
                    Ok(v) => println!("  = {}   (fuel used {})", v, 200_000 - fuel),
                    Err(e) => println!("  runtime error: {}", e),
                }
            }
        }
    }
}
