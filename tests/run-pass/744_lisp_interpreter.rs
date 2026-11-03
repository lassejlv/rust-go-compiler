// A small Lisp interpreter: reader, Rc-based values, lexical environments with
// Rc<RefCell<Env>>, lambdas with closures, tail-call loop for `if`/`begin`,
// builtins as fn pointers, define/set!/let, quote, and error reporting.
use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt;
use std::rc::Rc;

#[derive(Clone)]
enum V {
    Nil,
    Int(i64),
    Bool(bool),
    Sym(Rc<str>),
    List(Rc<Vec<V>>),
    Builtin(&'static str, fn(&[V]) -> Result<V, String>),
    Lambda(Rc<Lambda>),
}

struct Lambda { params: Vec<Rc<str>>, body: Vec<V>, env: Rc<RefCell<Env>> }

struct Env { vars: HashMap<Rc<str>, V>, parent: Option<Rc<RefCell<Env>>> }

impl Env {
    fn lookup(env: &Rc<RefCell<Env>>, k: &str) -> Option<V> {
        let e = env.borrow();
        match e.vars.get(k) { Some(v) => Some(v.clone()), None => e.parent.as_ref().and_then(|p| Env::lookup(p, k)) }
    }
    fn set(env: &Rc<RefCell<Env>>, k: &str, v: V) -> bool {
        let mut e = env.borrow_mut();
        if let Some(slot) = e.vars.get_mut(k) { *slot = v; return true; }
        match e.parent.clone() { Some(p) => { drop(e); Env::set(&p, k, v) } None => false }
    }
}

impl fmt::Display for V {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            V::Nil => write!(f, "nil"),
            V::Int(i) => write!(f, "{}", i),
            V::Bool(b) => write!(f, "{}", if *b { "#t" } else { "#f" }),
            V::Sym(s) => write!(f, "{}", s),
            V::List(l) => { write!(f, "(")?; for (i, x) in l.iter().enumerate() { if i > 0 { write!(f, " ")?; } write!(f, "{}", x)?; } write!(f, ")") }
            V::Builtin(n, _) => write!(f, "<builtin {}>", n),
            V::Lambda(l) => write!(f, "<lambda/{}>", l.params.len()),
        }
    }
}

fn tokenize(s: &str) -> Vec<String> { s.replace('(', " ( ").replace(')', " ) ").replace('\'', " ' ").split_whitespace().map(String::from).collect() }

fn read(toks: &[String], pos: &mut usize) -> Result<V, String> {
    let t = toks.get(*pos).ok_or("unexpected EOF")?;
    *pos += 1;
    match t.as_str() {
        "(" => {
            let mut items = vec![];
            loop {
                match toks.get(*pos).map(String::as_str) {
                    Some(")") => { *pos += 1; return Ok(V::List(Rc::new(items))); }
                    Some(_) => items.push(read(toks, pos)?),
                    None => return Err("missing )".into()),
                }
            }
        }
        ")" => Err("unexpected )".into()),
        "'" => Ok(V::List(Rc::new(vec![V::Sym("quote".into()), read(toks, pos)?]))),
        "#t" => Ok(V::Bool(true)),
        "#f" => Ok(V::Bool(false)),
        "nil" => Ok(V::Nil),
        _ => Ok(t.parse::<i64>().map(V::Int).unwrap_or_else(|_| V::Sym(t.as_str().into()))),
    }
}

fn ints(args: &[V]) -> Result<Vec<i64>, String> {
    args.iter().map(|a| match a { V::Int(i) => Ok(*i), o => Err(format!("not a number: {}", o)) }).collect()
}

fn builtins() -> Vec<(&'static str, fn(&[V]) -> Result<V, String>)> {
    vec![
        ("+", |a| Ok(V::Int(ints(a)?.iter().sum()))),
        ("*", |a| Ok(V::Int(ints(a)?.iter().product()))),
        ("-", |a| { let v = ints(a)?; Ok(V::Int(if v.len() == 1 { -v[0] } else { v[0] - v[1..].iter().sum::<i64>() })) }),
        ("/", |a| { let v = ints(a)?; if v[1] == 0 { Err("division by zero".into()) } else { Ok(V::Int(v[0] / v[1])) } }),
        ("<", |a| { let v = ints(a)?; Ok(V::Bool(v.windows(2).all(|w| w[0] < w[1]))) }),
        ("=", |a| { let v = ints(a)?; Ok(V::Bool(v.windows(2).all(|w| w[0] == w[1]))) }),
        ("list", |a| Ok(V::List(Rc::new(a.to_vec())))),
        ("car", |a| match a.first() { Some(V::List(l)) if !l.is_empty() => Ok(l[0].clone()), _ => Err("car of non-list".into()) }),
        ("cdr", |a| match a.first() { Some(V::List(l)) if !l.is_empty() => Ok(V::List(Rc::new(l[1..].to_vec()))), _ => Err("cdr of non-list".into()) }),
        ("cons", |a| match &a[1] { V::List(l) => { let mut v = vec![a[0].clone()]; v.extend(l.iter().cloned()); Ok(V::List(Rc::new(v))) } V::Nil => Ok(V::List(Rc::new(vec![a[0].clone()]))), _ => Err("cons onto non-list".into()) }),
        ("null?", |a| Ok(V::Bool(matches!(&a[0], V::Nil) || matches!(&a[0], V::List(l) if l.is_empty())))),
    ]
}

fn truthy(v: &V) -> bool { !matches!(v, V::Bool(false) | V::Nil) }

fn eval(mut x: V, mut env: Rc<RefCell<Env>>, depth: &mut usize) -> Result<V, String> {
    *depth += 1;
    loop {
        match x {
            V::Sym(s) => return Env::lookup(&env, &s).ok_or_else(|| format!("unbound symbol {}", s)),
            V::List(ref l) if l.is_empty() => return Ok(V::Nil),
            V::List(l) => {
                if let V::Sym(head) = &l[0] {
                    match &**head {
                        "quote" => return Ok(l[1].clone()),
                        "if" => {
                            let c = eval(l[1].clone(), env.clone(), depth)?;
                            x = if truthy(&c) { l[2].clone() } else { l.get(3).cloned().unwrap_or(V::Nil) };
                            continue;
                        }
                        "define" | "set!" => {
                            let V::Sym(name) = &l[1] else { return Err("bad define".into()) };
                            let v = eval(l[2].clone(), env.clone(), depth)?;
                            if &**head == "define" { env.borrow_mut().vars.insert(name.clone(), v); }
                            else if !Env::set(&env, name, v) { return Err(format!("set! of undefined {}", name)); }
                            return Ok(V::Nil);
                        }
                        "lambda" => {
                            let V::List(ps) = &l[1] else { return Err("bad params".into()) };
                            let params = ps.iter().map(|p| match p { V::Sym(s) => Ok(s.clone()), _ => Err("param not symbol".to_string()) }).collect::<Result<_, _>>()?;
                            return Ok(V::Lambda(Rc::new(Lambda { params, body: l[2..].to_vec(), env: env.clone() })));
                        }
                        "begin" => {
                            for e in &l[1..l.len() - 1] { eval(e.clone(), env.clone(), depth)?; }
                            x = l[l.len() - 1].clone();
                            continue;
                        }
                        "let" => {
                            let V::List(binds) = &l[1] else { return Err("bad let".into()) };
                            let new = Rc::new(RefCell::new(Env { vars: HashMap::new(), parent: Some(env.clone()) }));
                            for b in binds.iter() {
                                let V::List(pair) = b else { return Err("bad binding".into()) };
                                let V::Sym(n) = &pair[0] else { return Err("bad binding name".into()) };
                                let v = eval(pair[1].clone(), env.clone(), depth)?;
                                new.borrow_mut().vars.insert(n.clone(), v);
                            }
                            env = new;
                            x = V::List(Rc::new(std::iter::once(V::Sym("begin".into())).chain(l[2..].iter().cloned()).collect()));
                            continue;
                        }
                        _ => {}
                    }
                }
                let f = eval(l[0].clone(), env.clone(), depth)?;
                let args = l[1..].iter().map(|a| eval(a.clone(), env.clone(), depth)).collect::<Result<Vec<_>, _>>()?;
                match f {
                    V::Builtin(_, func) => return func(&args),
                    V::Lambda(lam) => {
                        if lam.params.len() != args.len() { return Err(format!("arity: expected {} got {}", lam.params.len(), args.len())); }
                        let vars = lam.params.iter().cloned().zip(args).collect();
                        env = Rc::new(RefCell::new(Env { vars, parent: Some(lam.env.clone()) }));
                        let n = lam.body.len();
                        for e in &lam.body[..n - 1] { eval(e.clone(), env.clone(), depth)?; }
                        x = lam.body[n - 1].clone();
                        continue;
                    }
                    other => return Err(format!("not callable: {}", other)),
                }
            }
            other => return Ok(other),
        }
    }
}

fn main() {
    let global = Rc::new(RefCell::new(Env { vars: HashMap::new(), parent: None }));
    for (n, f) in builtins() { global.borrow_mut().vars.insert(n.into(), V::Builtin(n, f)); }
    let program = [
        "(define fact (lambda (n) (if (< n 2) 1 (* n (fact (- n 1))))))",
        "(fact 20)",
        "(define make-counter (lambda () (let ((n 0)) (lambda () (set! n (+ n 1)) n))))",
        "(define c1 (make-counter))",
        "(define c2 (make-counter))",
        "(list (c1) (c1) (c1) (c2))",
        "(define map (lambda (f xs) (if (null? xs) '() (cons (f (car xs)) (map f (cdr xs))))))",
        "(map (lambda (x) (* x x)) '(1 2 3 4 5))",
        "(define loop (lambda (i acc) (if (= i 0) acc (loop (- i 1) (+ acc i)))))",
        "(loop 100000 0)",
        "(let ((x 2) (y 3)) (define z 10) (+ x y z))",
        "(define compose (lambda (f g) (lambda (x) (f (g x)))))",
        "((compose car cdr) '(a b c))",
        "'(nested (list (with) 3) #t #f nil)",
        "(/ 10 0)",
        "(undefined-fn 1)",
        "(car 5)",
        "((lambda (a b) a) 1)",
        "(set! nope 1)",
        "(1 2 3)",
        "(+ 1 (quote x))",
        "(begin (define a 5) (set! a (* a a)) a)",
    ];
    for src in program {
        let toks = tokenize(src);
        let mut pos = 0;
        let mut depth = 0;
        let res = read(&toks, &mut pos).and_then(|ast| eval(ast, global.clone(), &mut depth));
        match res {
            Ok(V::Nil) if src.starts_with("(define") => {}
            Ok(v) => println!("{} => {}  [evals {}]", src, v, depth),
            Err(e) => println!("{} => ERROR: {}", src, e),
        }
    }
    for bad in ["(+ 1 2", ")", ""] {
        let toks = tokenize(bad);
        println!("read {:?}: {:?}", bad, read(&toks, &mut 0).map(|v| v.to_string()));
    }
    println!("globals: {}", { let mut k: Vec<String> = global.borrow().vars.keys().map(|k| k.to_string()).filter(|k| k.chars().all(|c| c.is_alphabetic() || c == '-')).collect(); k.sort(); k.join(",") });
}
