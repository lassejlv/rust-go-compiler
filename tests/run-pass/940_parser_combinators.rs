// A small trait-based parser-combinator library and a JSON-ish parser built on it.
use std::collections::BTreeMap;
use std::fmt;

type PResult<'a, T> = Result<(T, &'a str), PError>;

#[derive(Debug, Clone, PartialEq)]
struct PError { at: usize, expected: String }

trait Parser<'a, T> {
    fn parse(&self, input: &'a str) -> PResult<'a, T>;

    fn map<U, F>(self, f: F) -> Map<Self, F, T>
    where
        Self: Sized,
        F: Fn(T) -> U,
    {
        Map { p: self, f, _t: std::marker::PhantomData }
    }

    fn and_then<U, P2, F>(self, f: F) -> AndThen<Self, F, T>
    where
        Self: Sized,
        P2: Parser<'a, U>,
        F: Fn(T) -> P2,
    {
        AndThen { p: self, f, _t: std::marker::PhantomData }
    }

    fn or<P2>(self, other: P2) -> Or<Self, P2>
    where
        Self: Sized,
        P2: Parser<'a, T>,
    {
        Or { a: self, b: other }
    }

    fn boxed(self) -> BoxedParser<'a, T>
    where
        Self: Sized + 'a,
    {
        BoxedParser { p: Box::new(self) }
    }
}

impl<'a, T, F> Parser<'a, T> for F
where
    F: Fn(&'a str) -> PResult<'a, T>,
{
    fn parse(&self, input: &'a str) -> PResult<'a, T> { self(input) }
}

struct BoxedParser<'a, T> { p: Box<dyn Parser<'a, T> + 'a> }
impl<'a, T> Parser<'a, T> for BoxedParser<'a, T> {
    fn parse(&self, input: &'a str) -> PResult<'a, T> { self.p.parse(input) }
}

struct Map<P, F, T> { p: P, f: F, _t: std::marker::PhantomData<T> }
impl<'a, P, F, T, U> Parser<'a, U> for Map<P, F, T>
where
    P: Parser<'a, T>,
    F: Fn(T) -> U,
{
    fn parse(&self, input: &'a str) -> PResult<'a, U> {
        self.p.parse(input).map(|(v, rest)| ((self.f)(v), rest))
    }
}

struct AndThen<P, F, T> { p: P, f: F, _t: std::marker::PhantomData<T> }
impl<'a, P, F, T, U, P2> Parser<'a, U> for AndThen<P, F, T>
where
    P: Parser<'a, T>,
    P2: Parser<'a, U>,
    F: Fn(T) -> P2,
{
    fn parse(&self, input: &'a str) -> PResult<'a, U> {
        let (v, rest) = self.p.parse(input)?;
        (self.f)(v).parse(rest)
    }
}

struct Or<A, B> { a: A, b: B }
impl<'a, T, A: Parser<'a, T>, B: Parser<'a, T>> Parser<'a, T> for Or<A, B> {
    fn parse(&self, input: &'a str) -> PResult<'a, T> {
        match self.a.parse(input) {
            Ok(r) => Ok(r),
            Err(e1) => match self.b.parse(input) {
                Ok(r) => Ok(r),
                Err(e2) => Err(if e2.at >= e1.at { e2 } else { e1 }),
            },
        }
    }
}

// Absolute offset tracking: we use a global base pointer per parse run via the original string length.
thread_local!(static TOTAL: std::cell::Cell<usize> = std::cell::Cell::new(0));
fn pos(rest: &str) -> usize { TOTAL.with(|t| t.get()) - rest.len() }

fn err<T>(rest: &str, what: &str) -> Result<T, PError> { Err(PError { at: pos(rest), expected: what.to_string() }) }

fn literal<'a>(lit: &'static str) -> impl Parser<'a, &'a str> {
    move |input: &'a str| {
        if input.starts_with(lit) { Ok((&input[..lit.len()], &input[lit.len()..])) } else { err(input, &format!("{:?}", lit)) }
    }
}

fn satisfy<'a, F: Fn(char) -> bool>(what: &'static str, f: F) -> impl Parser<'a, char> {
    move |input: &'a str| match input.chars().next() {
        Some(c) if f(c) => Ok((c, &input[c.len_utf8()..])),
        _ => err(input, what),
    }
}

fn many<'a, T, P: Parser<'a, T>>(p: P) -> impl Parser<'a, Vec<T>> {
    move |mut input: &'a str| {
        let mut out = Vec::new();
        while let Ok((v, rest)) = p.parse(input) {
            if rest.len() == input.len() { break; }
            out.push(v);
            input = rest;
        }
        Ok((out, input))
    }
}

fn many1<'a, T, P: Parser<'a, T>>(p: P, what: &'static str) -> impl Parser<'a, Vec<T>> {
    let m = many(p);
    move |input: &'a str| {
        let (v, rest) = m.parse(input)?;
        if v.is_empty() { err(input, what) } else { Ok((v, rest)) }
    }
}

fn ws<'a>() -> impl Parser<'a, ()> { many(satisfy("ws", char::is_whitespace)).map(|_| ()) }

fn token<'a, T, P: Parser<'a, T>>(p: P) -> impl Parser<'a, T> {
    let w = ws();
    move |input: &'a str| {
        let (_, r) = w.parse(input)?;
        let (v, r) = p.parse(r)?;
        let (_, r) = w.parse(r)?;
        Ok((v, r))
    }
}

fn sep_by<'a, T, S, P: Parser<'a, T>, Q: Parser<'a, S>>(p: P, sep: Q) -> impl Parser<'a, Vec<T>> {
    move |input: &'a str| {
        let mut out = Vec::new();
        let (first, mut rest) = match p.parse(input) { Ok(r) => r, Err(_) => return Ok((out, input)) };
        out.push(first);
        loop {
            match sep.parse(rest) {
                Ok((_, r)) => { let (v, r2) = p.parse(r)?; out.push(v); rest = r2; }
                Err(_) => return Ok((out, rest)),
            }
        }
    }
}

fn between<'a, A, T, B, PA: Parser<'a, A>, P: Parser<'a, T>, PB: Parser<'a, B>>(a: PA, p: P, b: PB) -> impl Parser<'a, T> {
    move |input: &'a str| {
        let (_, r) = a.parse(input)?;
        let (v, r) = p.parse(r)?;
        let (_, r) = b.parse(r)?;
        Ok((v, r))
    }
}

#[derive(Debug, Clone, PartialEq)]
enum Json { Null, Bool(bool), Num(f64), Str(String), Arr(Vec<Json>), Obj(BTreeMap<String, Json>) }

impl fmt::Display for Json {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Json::Null => write!(f, "null"),
            Json::Bool(b) => write!(f, "{}", b),
            Json::Num(n) => write!(f, "{}", n),
            Json::Str(s) => write!(f, "{:?}", s),
            Json::Arr(v) => {
                write!(f, "[")?;
                for (i, x) in v.iter().enumerate() { if i > 0 { write!(f, ",")?; } write!(f, "{}", x)?; }
                write!(f, "]")
            }
            Json::Obj(m) => {
                write!(f, "{{")?;
                for (i, (k, v)) in m.iter().enumerate() { if i > 0 { write!(f, ",")?; } write!(f, "{:?}:{}", k, v)?; }
                write!(f, "}}")
            }
        }
    }
}

fn number<'a>() -> impl Parser<'a, f64> {
    move |input: &'a str| {
        let (neg, r) = match literal("-").parse(input) { Ok((_, r)) => (true, r), Err(_) => (false, input) };
        let (digits, r) = many1(satisfy("digit", |c| c.is_ascii_digit()), "digit").parse(r)?;
        let (frac, r) = match literal(".").parse(r) {
            Ok((_, r2)) => { let (d, r3) = many1(satisfy("digit", |c| c.is_ascii_digit()), "fraction digit").parse(r2)?; (d, r3) }
            Err(_) => (vec![], r),
        };
        let mut s: String = digits.into_iter().collect();
        if !frac.is_empty() { s.push('.'); s.extend(frac); }
        let v: f64 = s.parse().unwrap();
        Ok((if neg { -v } else { v }, r))
    }
}

fn string_lit<'a>() -> impl Parser<'a, String> {
    between(literal("\""), many(satisfy("string char", |c| c != '"')), literal("\"")).map(|cs: Vec<char>| cs.into_iter().collect())
}

fn json_value<'a>() -> BoxedParser<'a, Json> {
    let null = literal("null").map(|_| Json::Null);
    let t = literal("true").map(|_| Json::Bool(true));
    let f = literal("false").map(|_| Json::Bool(false));
    let num = number().map(Json::Num);
    let s = string_lit().map(Json::Str);
    let arr = |input: &'a str| {
        between(token(literal("[")), sep_by(json_value(), token(literal(","))), token(literal("]"))).map(Json::Arr).parse(input)
    };
    let obj = |input: &'a str| {
        let pair = string_lit().and_then(|k: String| {
            token(literal(":")).and_then(move |_| { let k = k.clone(); json_value().map(move |v| (k.clone(), v)) })
        });
        between(token(literal("{")), sep_by(token(pair), token(literal(","))), token(literal("}")))
            .map(|ps: Vec<(String, Json)>| Json::Obj(ps.into_iter().collect()))
            .parse(input)
    };
    token(null.or(t).or(f).or(num).or(s).or(arr).or(obj)).boxed()
}

fn parse_json(s: &str) -> Result<Json, PError> {
    TOTAL.with(|t| t.set(s.len()));
    let (v, rest) = json_value().parse(s)?;
    if !rest.is_empty() { return err(rest, "end of input"); }
    Ok(v)
}

fn main() {
    let inputs = [
        "null",
        " true ",
        "-12.5",
        "\"hi there\"",
        "[1, 2, [3, []], \"x\"]",
        "{\"b\": 1, \"a\": [true, false, null], \"c\": {\"d\": \"e\"}}",
        "[1, 2",
        "{\"k\" 1}",
        "[1.]",
        "tru",
        "[] x",
    ];
    for i in inputs.iter() {
        match parse_json(i) {
            Ok(j) => println!("OK  {:<45} => {}", i, j),
            Err(e) => println!("ERR {:<45} => at {} expected {}", i, e.at, e.expected),
        }
    }
    TOTAL.with(|t| t.set(9));
    let words = sep_by(many1(satisfy("letter", |c| c.is_alphabetic()), "word").map(|v: Vec<char>| v.len()), literal(","));
    println!("{:?}", words.parse("ab,cde,f"));
    let kv = literal("x").and_then(|_| number()).map(|n| n * 2.0);
    TOTAL.with(|t| t.set(4));
    println!("{:?}", kv.parse("x21;"));
}
