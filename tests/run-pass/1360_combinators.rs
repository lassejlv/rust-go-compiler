// Parser combinators in the "nom" style: a Parser trait implemented for
// closures, combinators returning `impl Parser`, deeply nested generic types,
// boxed dynamic parsers for recursion, and grammars for arithmetic, INI files
// and a small list language.
use std::collections::BTreeMap;
use std::fmt::Debug;
use std::rc::Rc;

#[derive(Debug, Clone, PartialEq)]
struct PErr {
    pos: usize,
    expected: Vec<String>,
}

type PResult<'a, T> = Result<(&'a str, T), PErr>;

trait Parser<'a, T> {
    fn parse(&self, input: &'a str) -> PResult<'a, T>;

    fn map<U, F: Fn(T) -> U>(self, f: F) -> Map<Self, F, T>
    where
        Self: Sized,
    {
        Map { p: self, f, _t: std::marker::PhantomData }
    }
}

impl<'a, T, F> Parser<'a, T> for F
where
    F: Fn(&'a str) -> PResult<'a, T>,
{
    fn parse(&self, input: &'a str) -> PResult<'a, T> {
        self(input)
    }
}

struct Map<P, F, T> {
    p: P,
    f: F,
    _t: std::marker::PhantomData<T>,
}

impl<'a, P, F, T, U> Parser<'a, U> for Map<P, F, T>
where
    P: Parser<'a, T>,
    F: Fn(T) -> U,
{
    fn parse(&self, input: &'a str) -> PResult<'a, U> {
        self.p.parse(input).map(|(rest, v)| (rest, (self.f)(v)))
    }
}

struct BoxedParser<'a, T> {
    p: Rc<dyn Parser<'a, T> + 'a>,
}

impl<'a, T> Clone for BoxedParser<'a, T> {
    fn clone(&self) -> Self {
        BoxedParser { p: self.p.clone() }
    }
}

impl<'a, T> BoxedParser<'a, T> {
    fn new<P: Parser<'a, T> + 'a>(p: P) -> Self {
        BoxedParser { p: Rc::new(p) }
    }
}

impl<'a, T> Parser<'a, T> for BoxedParser<'a, T> {
    fn parse(&self, input: &'a str) -> PResult<'a, T> {
        self.p.parse(input)
    }
}

// position is tracked as "remaining length" relative to the original input
fn err<T>(input: &str, what: &str) -> Result<T, PErr> {
    Err(PErr { pos: input.len(), expected: vec![what.to_string()] })
}

fn tag<'a>(t: &'static str) -> impl Parser<'a, &'a str> {
    move |input: &'a str| match input.strip_prefix(t) {
        Some(rest) => Ok((rest, &input[..t.len()])),
        None => err(input, &format!("{:?}", t)),
    }
}

fn satisfy<'a>(what: &'static str, pred: impl Fn(char) -> bool) -> impl Parser<'a, char> {
    move |input: &'a str| match input.chars().next() {
        Some(c) if pred(c) => Ok((&input[c.len_utf8()..], c)),
        _ => err(input, what),
    }
}

fn take_while1<'a>(what: &'static str, pred: impl Fn(char) -> bool) -> impl Parser<'a, &'a str> {
    move |input: &'a str| {
        let end = input.char_indices().find(|&(_, c)| !pred(c)).map_or(input.len(), |(i, _)| i);
        if end == 0 {
            err(input, what)
        } else {
            Ok((&input[end..], &input[..end]))
        }
    }
}

fn ws<'a>() -> impl Parser<'a, ()> {
    |input: &'a str| Ok((input.trim_start_matches([' ', '\t']), ()))
}

fn pair<'a, A, B>(pa: impl Parser<'a, A>, pb: impl Parser<'a, B>) -> impl Parser<'a, (A, B)> {
    move |input| {
        let (r, a) = pa.parse(input)?;
        let (r, b) = pb.parse(r)?;
        Ok((r, (a, b)))
    }
}

fn left<'a, A, B>(pa: impl Parser<'a, A>, pb: impl Parser<'a, B>) -> impl Parser<'a, A> {
    pair(pa, pb).map(|(a, _)| a)
}

fn right<'a, A, B>(pa: impl Parser<'a, A>, pb: impl Parser<'a, B>) -> impl Parser<'a, B> {
    pair(pa, pb).map(|(_, b)| b)
}

fn token<'a, T>(p: impl Parser<'a, T>) -> impl Parser<'a, T> {
    right(ws(), left(p, ws()))
}

fn alt<'a, T>(pa: impl Parser<'a, T>, pb: impl Parser<'a, T>) -> impl Parser<'a, T> {
    move |input| match pa.parse(input) {
        Ok(v) => Ok(v),
        Err(e1) => match pb.parse(input) {
            Ok(v) => Ok(v),
            Err(e2) => Err(match e1.pos.cmp(&e2.pos) {
                // smaller remaining length = got further
                std::cmp::Ordering::Less => e1,
                std::cmp::Ordering::Greater => e2,
                std::cmp::Ordering::Equal => PErr { pos: e1.pos, expected: e1.expected.into_iter().chain(e2.expected).collect() },
            }),
        },
    }
}

fn many0<'a, T>(p: impl Parser<'a, T>) -> impl Parser<'a, Vec<T>> {
    move |mut input| {
        let mut out = Vec::new();
        while let Ok((rest, v)) = p.parse(input) {
            if rest.len() == input.len() {
                break;
            }
            out.push(v);
            input = rest;
        }
        Ok((input, out))
    }
}

fn sep_by<'a, T, S>(p: impl Parser<'a, T>, sep: impl Parser<'a, S>) -> impl Parser<'a, Vec<T>> {
    move |input| {
        let mut out = Vec::new();
        let (mut input, first) = match p.parse(input) {
            Ok(v) => v,
            Err(_) => return Ok((input, out)),
        };
        out.push(first);
        loop {
            let Ok((r, _)) = sep.parse(input) else { break };
            let (r, v) = p.parse(r)?;
            out.push(v);
            input = r;
        }
        Ok((input, out))
    }
}

fn opt<'a, T>(p: impl Parser<'a, T>) -> impl Parser<'a, Option<T>> {
    move |input| match p.parse(input) {
        Ok((r, v)) => Ok((r, Some(v))),
        Err(_) => Ok((input, None)),
    }
}

fn eof<'a>() -> impl Parser<'a, ()> {
    |input: &'a str| if input.is_empty() { Ok((input, ())) } else { err(input, "end of input") }
}

fn number<'a>() -> impl Parser<'a, f64> {
    let digits = take_while1("digit", |c| c.is_ascii_digit());
    let frac = opt(right(tag("."), take_while1("digit", |c| c.is_ascii_digit())));
    token(pair(pair(opt(tag("-")), digits), frac).map(|((neg, int), frac): ((Option<&str>, &str), Option<&str>)| {
        let s = format!("{}{}.{}", neg.unwrap_or(""), int, frac.unwrap_or("0"));
        s.parse::<f64>().unwrap()
    }))
}

// ------------- arithmetic with a recursive boxed parser --------------
#[derive(Debug, Clone, PartialEq)]
enum Ast {
    Num(f64),
    Var(String),
    Neg(Box<Ast>),
    Bin(char, Box<Ast>, Box<Ast>),
    Call(String, Vec<Ast>),
}

fn eval(a: &Ast, vars: &BTreeMap<&str, f64>) -> Result<f64, String> {
    Ok(match a {
        Ast::Num(n) => *n,
        Ast::Var(v) => *vars.get(v.as_str()).ok_or(format!("unknown variable {}", v))?,
        Ast::Neg(x) => -eval(x, vars)?,
        Ast::Bin(op, l, r) => {
            let (l, r) = (eval(l, vars)?, eval(r, vars)?);
            match op {
                '+' => l + r,
                '-' => l - r,
                '*' => l * r,
                '/' if r == 0.0 => return Err("division by zero".into()),
                '/' => l / r,
                '^' => l.powf(r),
                _ => unreachable!(),
            }
        }
        Ast::Call(f, args) => {
            let vs = args.iter().map(|x| eval(x, vars)).collect::<Result<Vec<_>, _>>()?;
            match (f.as_str(), vs.as_slice()) {
                ("max", [a, rest @ ..]) => rest.iter().fold(*a, |m, x| m.max(*x)),
                ("min", [a, rest @ ..]) => rest.iter().fold(*a, |m, x| m.min(*x)),
                ("sqrt", [x]) => x.sqrt(),
                ("abs", [x]) => x.abs(),
                (f, args) => return Err(format!("bad call {}({} args)", f, args.len())),
            }
        }
    })
}

fn chainl<'a>(operand: BoxedParser<'a, Ast>, ops: &'static str) -> BoxedParser<'a, Ast> {
    let op = token(satisfy("operator", move |c| ops.contains(c)));
    let rest = many0(pair(op, operand.clone()));
    BoxedParser::new(pair(operand, rest).map(|(first, rest): (Ast, Vec<(char, Ast)>)| rest.into_iter().fold(first, |l, (op, r)| Ast::Bin(op, Box::new(l), Box::new(r)))))
}

fn expr_parser<'a>() -> BoxedParser<'a, Ast> {
    // late-bound recursion through a shared cell
    let slot: Rc<std::cell::RefCell<Option<BoxedParser<'a, Ast>>>> = Rc::new(std::cell::RefCell::new(None));
    let s2 = slot.clone();
    let expr_ref = BoxedParser::new(move |input: &'a str| s2.borrow().as_ref().expect("tied").parse(input));
    let ident = token(take_while1("identifier", |c| c.is_ascii_alphabetic() || c == '_'));
    let call = pair(ident, opt(right(token(tag("(")), left(sep_by(expr_ref.clone(), token(tag(","))), token(tag(")")))))).map(|(name, args): (&str, Option<Vec<Ast>>)| match args {
        Some(a) => Ast::Call(name.to_string(), a),
        None => Ast::Var(name.to_string()),
    });
    let paren = right(token(tag("(")), left(expr_ref.clone(), token(tag(")"))));
    let atom = BoxedParser::new(alt(alt(number().map(Ast::Num), call), paren));
    let atom2 = atom.clone();
    let unary = BoxedParser::new(alt(right(token(tag("-")), atom.clone()).map(|a| Ast::Neg(Box::new(a))), atom2));
    // right-assoc power
    let pow_slot: Rc<std::cell::RefCell<Option<BoxedParser<'a, Ast>>>> = Rc::new(std::cell::RefCell::new(None));
    let p2 = pow_slot.clone();
    let pow_ref = BoxedParser::new(move |i: &'a str| p2.borrow().as_ref().unwrap().parse(i));
    let power = BoxedParser::new(pair(unary.clone(), opt(right(token(tag("^")), pow_ref))).map(|(b, e): (Ast, Option<Ast>)| match e {
        Some(e) => Ast::Bin('^', Box::new(b), Box::new(e)),
        None => b,
    }));
    *pow_slot.borrow_mut() = Some(power.clone());
    let term = chainl(power, "*/");
    let sum = chainl(term, "+-");
    *slot.borrow_mut() = Some(sum.clone());
    sum
}

// ------------- INI files --------------
type Ini = BTreeMap<String, BTreeMap<String, String>>;

fn ini_parser<'a>() -> impl Parser<'a, Ini> {
    let nl = |i: &'a str| -> PResult<'a, ()> {
        let t = i.trim_start_matches([' ', '\t']);
        let t = match t.strip_prefix(';') {
            Some(c) => c.trim_start_matches(|c| c != '\n'),
            None => t,
        };
        match t.strip_prefix('\n') {
            Some(r) => Ok((r, ())),
            None if t.is_empty() => Ok((t, ())),
            None => err(t, "newline"),
        }
    };
    let blank = many0(nl);
    let name = |i| take_while1("name", |c: char| c.is_alphanumeric() || c == '_' || c == '.').parse(i);
    let header = left(right(pair(blank, tag("[")), name), pair(tag("]"), nl));
    let value = take_while1("value", |c| c != '\n' && c != ';');
    let kv = left(pair(left(token(name), tag("=")), token(value)), pair(nl, many0(nl)));
    let section = pair(header, many0(kv));
    left(many0(section), pair(many0(nl), eof())).map(|secs: Vec<(&str, Vec<(&str, &str)>)>| {
        let mut ini = Ini::new();
        for (n, kvs) in secs {
            let e = ini.entry(n.to_string()).or_default();
            for (k, v) in kvs {
                e.insert(k.to_string(), v.trim_end().to_string());
            }
        }
        ini
    })
}

fn describe_err(src: &str, e: &PErr) -> String {
    let at = src.len() - e.pos;
    let mut exp = e.expected.clone();
    exp.sort();
    exp.dedup();
    format!("at offset {} (near {:?}): expected {}", at, &src[at..src.len().min(at + 6)], exp.join(" or "))
}

fn main() {
    let p = expr_parser();
    let full = left(p.clone(), eof());
    let mut vars = BTreeMap::new();
    vars.insert("x", 3.0);
    vars.insert("pi", 3.14159);
    let cases = [
        "1 + 2 * 3",
        "(1 + 2) * 3",
        "2 ^ 3 ^ 2",
        "-x ^ 2",
        "10 / 4 - 1.5",
        "max(1, x * 2, 4) + min(7, 8)",
        "sqrt(16) * abs(-2.5)",
        "2 * pi * x",
        "1 / (x - 3)",
        "foo(1)",
        "y + 1",
        "1 + ",
        "(1 + 2",
        "3 $ 4",
        "max()",
    ];
    for c in cases {
        match full.parse(c) {
            Ok((_, ast)) => match eval(&ast, &vars) {
                Ok(v) => println!("{:<30} = {}", c, v),
                Err(e) => println!("{:<30} eval error: {}", c, e),
            },
            Err(e) => println!("{:<30} parse error {}", c, describe_err(c, &e)),
        }
    }
    let (_, ast) = full.parse("1 - 2 - 3 * -(4)").unwrap();
    println!("{:?}", ast);

    let ini_src = "; global comment\n\n[server]\nhost = example.com\nport=8080 ; inline\n\n[db.main]\nuser = admin\npass = s3cr3t\n[empty]\n";
    match ini_parser().parse(ini_src) {
        Ok((rest, ini)) => {
            println!("ini rest={:?}", rest);
            for (sec, kv) in &ini {
                println!("[{}] {:?}", sec, kv);
            }
        }
        Err(e) => println!("ini error {}", describe_err(ini_src, &e)),
    }
    let bad_ini = "[ok]\na=1\n[bad\nb=2\n";
    match ini_parser().parse(bad_ini) {
        Ok((_, ini)) => println!("bad ini parsed?? {:?}", ini),
        Err(e) => println!("bad ini: {}", describe_err(bad_ini, &e)),
    }
    let list = sep_by(number(), token(tag(",")));
    println!("{:?}", list.parse("1, 2.5,-3 ,4 rest"));
    println!("{:?}", left(list, eof()).parse("1,2,"));
    let words = many0(token(take_while1("word", char::is_alphabetic)));
    println!("{:?}", words.parse("  héllo wörld 42"));
    let kw = alt(alt(tag("let"), tag("letrec")), tag("fn"));
    println!("{:?} {:?}", kw.parse("letrec x"), kw.parse("var"));
}
