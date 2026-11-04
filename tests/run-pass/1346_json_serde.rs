// JSON value model, parser, pretty printer and a hand-written serde-like
// Serialize/Deserialize trait pair with blanket impls and a derive-ish macro.
use std::borrow::Cow;
use std::collections::{BTreeMap, HashMap};
use std::fmt::{self, Write};

#[derive(Debug, Clone, PartialEq)]
enum Json {
    Null,
    Bool(bool),
    Num(f64),
    Str(String),
    Arr(Vec<Json>),
    Obj(BTreeMap<String, Json>),
}

#[derive(Debug, Clone, PartialEq)]
struct ParseError {
    line: usize,
    col: usize,
    msg: Cow<'static, str>,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}:{}: {}", self.line, self.col, self.msg)
    }
}

struct P<'a> {
    s: &'a str,
    i: usize,
}

impl<'a> P<'a> {
    fn err<T>(&self, msg: impl Into<Cow<'static, str>>) -> Result<T, ParseError> {
        let before = &self.s[..self.i];
        let line = before.matches('\n').count() + 1;
        let col = before.rfind('\n').map_or(self.i, |p| self.i - p - 1) + 1;
        Err(ParseError { line, col, msg: msg.into() })
    }
    fn ws(&mut self) {
        let rest = &self.s[self.i..];
        self.i += rest.len() - rest.trim_start().len();
    }
    fn peek(&self) -> Option<char> {
        self.s[self.i..].chars().next()
    }
    fn lit(&mut self, w: &str, v: Json) -> Result<Json, ParseError> {
        if self.s[self.i..].starts_with(w) {
            self.i += w.len();
            Ok(v)
        } else {
            self.err(format!("expected `{}`", w))
        }
    }
    fn value(&mut self) -> Result<Json, ParseError> {
        self.ws();
        match self.peek() {
            None => self.err("unexpected end of input"),
            Some('n') => self.lit("null", Json::Null),
            Some('t') => self.lit("true", Json::Bool(true)),
            Some('f') => self.lit("false", Json::Bool(false)),
            Some('"') => Ok(Json::Str(self.string()?)),
            Some('[') => {
                self.i += 1;
                let mut v = Vec::new();
                self.ws();
                if self.peek() == Some(']') {
                    self.i += 1;
                    return Ok(Json::Arr(v));
                }
                loop {
                    v.push(self.value()?);
                    self.ws();
                    match self.peek() {
                        Some(',') => self.i += 1,
                        Some(']') => {
                            self.i += 1;
                            return Ok(Json::Arr(v));
                        }
                        _ => return self.err("expected `,` or `]`"),
                    }
                }
            }
            Some('{') => {
                self.i += 1;
                let mut m = BTreeMap::new();
                self.ws();
                if self.peek() == Some('}') {
                    self.i += 1;
                    return Ok(Json::Obj(m));
                }
                loop {
                    self.ws();
                    if self.peek() != Some('"') {
                        return self.err("expected string key");
                    }
                    let k = self.string()?;
                    self.ws();
                    if self.peek() != Some(':') {
                        return self.err("expected `:`");
                    }
                    self.i += 1;
                    let v = self.value()?;
                    if m.insert(k.clone(), v).is_some() {
                        return self.err(format!("duplicate key {:?}", k));
                    }
                    self.ws();
                    match self.peek() {
                        Some(',') => self.i += 1,
                        Some('}') => {
                            self.i += 1;
                            return Ok(Json::Obj(m));
                        }
                        _ => return self.err("expected `,` or `}`"),
                    }
                }
            }
            Some(c) if c == '-' || c.is_ascii_digit() => {
                let start = self.i;
                let end = self.s[start..].find(|c: char| !(c.is_ascii_digit() || "+-.eE".contains(c))).map_or(self.s.len(), |e| start + e);
                self.i = end;
                match self.s[start..end].parse::<f64>() {
                    Ok(n) => Ok(Json::Num(n)),
                    Err(e) => {
                        self.i = start;
                        self.err(format!("bad number: {}", e))
                    }
                }
            }
            Some(c) => self.err(format!("unexpected character {:?}", c)),
        }
    }
    fn string(&mut self) -> Result<String, ParseError> {
        self.i += 1;
        let mut out = String::new();
        let mut chars = self.s[self.i..].char_indices();
        while let Some((off, c)) = chars.next() {
            match c {
                '"' => {
                    self.i += off + 1;
                    return Ok(out);
                }
                '\\' => {
                    let (_, e) = chars.next().ok_or(ParseError { line: 0, col: 0, msg: "eof in escape".into() })?;
                    match e {
                        'n' => out.push('\n'),
                        't' => out.push('\t'),
                        '"' => out.push('"'),
                        '\\' => out.push('\\'),
                        '/' => out.push('/'),
                        'u' => {
                            let hex: String = chars.by_ref().take(4).map(|(_, c)| c).collect();
                            let cp = u32::from_str_radix(&hex, 16).map_err(|e| ParseError { line: 0, col: 0, msg: format!("bad \\u escape {:?}: {}", hex, e).into() })?;
                            out.push(char::from_u32(cp).unwrap_or('\u{FFFD}'));
                        }
                        other => {
                            self.i += off;
                            return self.err(format!("bad escape \\{}", other));
                        }
                    }
                }
                c => out.push(c),
            }
        }
        self.err("unterminated string")
    }
}

fn parse(s: &str) -> Result<Json, ParseError> {
    let mut p = P { s, i: 0 };
    let v = p.value()?;
    p.ws();
    if p.i != s.len() {
        return p.err("trailing characters");
    }
    Ok(v)
}

fn escape(s: &str, out: &mut String) {
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => write!(out, "\\u{:04x}", c as u32).unwrap(),
            c => out.push(c),
        }
    }
    out.push('"');
}

impl Json {
    fn write(&self, out: &mut String, indent: Option<usize>, level: usize) {
        let nl = |out: &mut String, lvl: usize| {
            if let Some(w) = indent {
                out.push('\n');
                out.extend(std::iter::repeat(' ').take(w * lvl));
            }
        };
        match self {
            Json::Null => out.push_str("null"),
            Json::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
            Json::Num(n) => {
                if n.fract() == 0.0 && n.abs() < 1e16 {
                    write!(out, "{}", *n as i64).unwrap()
                } else {
                    write!(out, "{}", n).unwrap()
                }
            }
            Json::Str(s) => escape(s, out),
            Json::Arr(v) if v.is_empty() => out.push_str("[]"),
            Json::Arr(v) => {
                out.push('[');
                for (i, x) in v.iter().enumerate() {
                    if i > 0 {
                        out.push(',');
                    }
                    nl(out, level + 1);
                    x.write(out, indent, level + 1);
                }
                nl(out, level);
                out.push(']');
            }
            Json::Obj(m) if m.is_empty() => out.push_str("{}"),
            Json::Obj(m) => {
                out.push('{');
                for (i, (k, v)) in m.iter().enumerate() {
                    if i > 0 {
                        out.push(',');
                    }
                    nl(out, level + 1);
                    escape(k, out);
                    out.push(':');
                    if indent.is_some() {
                        out.push(' ');
                    }
                    v.write(out, indent, level + 1);
                }
                nl(out, level);
                out.push('}');
            }
        }
    }
    fn pretty(&self) -> String {
        let mut s = String::new();
        self.write(&mut s, Some(2), 0);
        s
    }
    fn get(&self, path: &str) -> Option<&Json> {
        path.split('.').filter(|p| !p.is_empty()).try_fold(self, |cur, seg| match cur {
            Json::Obj(m) => m.get(seg),
            Json::Arr(v) => seg.parse::<usize>().ok().and_then(|i| v.get(i)),
            _ => None,
        })
    }
}

impl fmt::Display for Json {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut s = String::new();
        self.write(&mut s, if f.alternate() { Some(2) } else { None }, 0);
        f.write_str(&s)
    }
}

// ---------------- serde-like layer ----------------
trait Serialize {
    fn to_json(&self) -> Json;
}

#[derive(Debug)]
enum DeError {
    Type { expected: &'static str, got: String },
    Missing(&'static str),
    Context(String, Box<DeError>),
}

impl fmt::Display for DeError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            DeError::Type { expected, got } => write!(f, "expected {}, got {}", expected, got),
            DeError::Missing(k) => write!(f, "missing field `{}`", k),
            DeError::Context(c, inner) => write!(f, "{}: {}", c, inner),
        }
    }
}

trait Deserialize: Sized {
    fn from_json(j: &Json) -> Result<Self, DeError>;
}

fn kind(j: &Json) -> String {
    match j {
        Json::Null => "null".into(),
        Json::Bool(_) => "bool".into(),
        Json::Num(n) => format!("number {}", n),
        Json::Str(s) => format!("string {:?}", s),
        Json::Arr(v) => format!("array of {}", v.len()),
        Json::Obj(_) => "object".into(),
    }
}

macro_rules! num_impl {
    ($($t:ty),*) => {$(
        impl Serialize for $t {
            fn to_json(&self) -> Json { Json::Num(*self as f64) }
        }
        impl Deserialize for $t {
            fn from_json(j: &Json) -> Result<Self, DeError> {
                match j {
                    Json::Num(n) if n.fract() == 0.0 || stringify!($t).starts_with('f') => Ok(*n as $t),
                    other => Err(DeError::Type { expected: stringify!($t), got: kind(other) }),
                }
            }
        }
    )*};
}
num_impl!(u8, u32, i32, i64, usize, f64);

impl Serialize for bool {
    fn to_json(&self) -> Json {
        Json::Bool(*self)
    }
}
impl Deserialize for bool {
    fn from_json(j: &Json) -> Result<Self, DeError> {
        if let Json::Bool(b) = j {
            Ok(*b)
        } else {
            Err(DeError::Type { expected: "bool", got: kind(j) })
        }
    }
}
impl Serialize for String {
    fn to_json(&self) -> Json {
        Json::Str(self.clone())
    }
}
impl Serialize for str {
    fn to_json(&self) -> Json {
        Json::Str(self.to_owned())
    }
}
impl Deserialize for String {
    fn from_json(j: &Json) -> Result<Self, DeError> {
        match j {
            Json::Str(s) => Ok(s.clone()),
            _ => Err(DeError::Type { expected: "string", got: kind(j) }),
        }
    }
}
impl<T: Serialize> Serialize for Vec<T> {
    fn to_json(&self) -> Json {
        Json::Arr(self.iter().map(Serialize::to_json).collect())
    }
}
impl<T: Serialize + ?Sized> Serialize for &T {
    fn to_json(&self) -> Json {
        (**self).to_json()
    }
}
impl<T: Serialize> Serialize for [T] {
    fn to_json(&self) -> Json {
        Json::Arr(self.iter().map(|x| x.to_json()).collect())
    }
}
impl<T: Deserialize> Deserialize for Vec<T> {
    fn from_json(j: &Json) -> Result<Self, DeError> {
        match j {
            Json::Arr(v) => v.iter().enumerate().map(|(i, x)| T::from_json(x).map_err(|e| DeError::Context(format!("[{}]", i), Box::new(e)))).collect(),
            _ => Err(DeError::Type { expected: "array", got: kind(j) }),
        }
    }
}
impl<T: Serialize> Serialize for Option<T> {
    fn to_json(&self) -> Json {
        self.as_ref().map_or(Json::Null, T::to_json)
    }
}
impl<T: Deserialize> Deserialize for Option<T> {
    fn from_json(j: &Json) -> Result<Self, DeError> {
        match j {
            Json::Null => Ok(None),
            j => T::from_json(j).map(Some),
        }
    }
}
impl<V: Serialize> Serialize for HashMap<String, V> {
    fn to_json(&self) -> Json {
        Json::Obj(self.iter().map(|(k, v)| (k.clone(), v.to_json())).collect())
    }
}
impl<V: Deserialize> Deserialize for HashMap<String, V> {
    fn from_json(j: &Json) -> Result<Self, DeError> {
        match j {
            Json::Obj(m) => m.iter().map(|(k, v)| Ok((k.clone(), V::from_json(v).map_err(|e| DeError::Context(k.clone(), Box::new(e)))?))).collect(),
            _ => Err(DeError::Type { expected: "object", got: kind(j) }),
        }
    }
}
impl<A: Serialize, B: Serialize> Serialize for (A, B) {
    fn to_json(&self) -> Json {
        Json::Arr(vec![self.0.to_json(), self.1.to_json()])
    }
}
impl<A: Deserialize, B: Deserialize> Deserialize for (A, B) {
    fn from_json(j: &Json) -> Result<Self, DeError> {
        match j {
            Json::Arr(v) if v.len() == 2 => Ok((A::from_json(&v[0])?, B::from_json(&v[1])?)),
            _ => Err(DeError::Type { expected: "pair", got: kind(j) }),
        }
    }
}

macro_rules! record {
    ($name:ident { $($field:ident : $ty:ty),* $(,)? }) => {
        #[derive(Debug, Clone, PartialEq)]
        struct $name { $($field: $ty),* }
        impl Serialize for $name {
            fn to_json(&self) -> Json {
                let mut m = BTreeMap::new();
                $( m.insert(stringify!($field).to_string(), self.$field.to_json()); )*
                Json::Obj(m)
            }
        }
        impl Deserialize for $name {
            fn from_json(j: &Json) -> Result<Self, DeError> {
                let m = match j {
                    Json::Obj(m) => m,
                    other => return Err(DeError::Type { expected: stringify!($name), got: kind(other) }),
                };
                Ok($name { $(
                    $field: {
                        let v = m.get(stringify!($field)).unwrap_or(&Json::Null);
                        <$ty>::from_json(v).map_err(|e| match (e, v) {
                            (DeError::Type { .. }, Json::Null) => DeError::Missing(stringify!($field)),
                            (e, _) => DeError::Context(format!("{}.{}", stringify!($name), stringify!($field)), Box::new(e)),
                        })?
                    },
                )* })
            }
        }
    };
}

record!(Address { street: String, zip: u32 });
record!(Person { name: String, age: u8, emails: Vec<String>, address: Option<Address>, scores: HashMap<String, f64>, pos: (i32, i32) });

#[derive(Debug, Clone, PartialEq)]
enum Shape {
    Circle { r: f64 },
    Rect { w: f64, h: f64 },
    Unit,
}

impl Serialize for Shape {
    fn to_json(&self) -> Json {
        let (tag, fields): (&str, Vec<(&str, f64)>) = match self {
            Shape::Circle { r } => ("circle", vec![("r", *r)]),
            Shape::Rect { w, h } => ("rect", vec![("w", *w), ("h", *h)]),
            Shape::Unit => ("unit", vec![]),
        };
        let mut m: BTreeMap<String, Json> = fields.into_iter().map(|(k, v)| (k.to_string(), Json::Num(v))).collect();
        m.insert("type".into(), Json::Str(tag.into()));
        Json::Obj(m)
    }
}
impl Deserialize for Shape {
    fn from_json(j: &Json) -> Result<Self, DeError> {
        let f = |k: &'static str| -> Result<f64, DeError> { j.get(k).ok_or(DeError::Missing(k)).and_then(f64::from_json) };
        match j.get("type") {
            Some(Json::Str(t)) => match t.as_str() {
                "circle" => Ok(Shape::Circle { r: f("r")? }),
                "rect" => Ok(Shape::Rect { w: f("w")?, h: f("h")? }),
                "unit" => Ok(Shape::Unit),
                other => Err(DeError::Type { expected: "shape tag", got: other.to_string() }),
            },
            Some(other) => Err(DeError::Type { expected: "string tag", got: kind(other) }),
            None => Err(DeError::Missing("type")),
        }
    }
}

fn roundtrip<T: Serialize + Deserialize + PartialEq + fmt::Debug>(v: &T) -> bool {
    let text = v.to_json().to_string();
    let back = parse(&text).map_err(|e| e.to_string()).and_then(|j| T::from_json(&j).map_err(|e| e.to_string()));
    match back {
        Ok(b) => b == *v,
        Err(e) => {
            println!("roundtrip error: {}", e);
            false
        }
    }
}

fn main() {
    let doc = r#"
    {
        "name": "Ada Lovelace", "age": 36,
        "emails": ["ada@example.com", "countess@analytical.engine"],
        "address": {"street": "12 St James's Sq\nLondon", "zip": 10001},
        "scores": {"math": 99.5, "poetry": 71},
        "pos": [-3, 4],
        "extra": [true, false, null, 1e3, -0.25, "\u00e9\t\"q\""]
    }"#;
    let j = parse(doc).expect("parse");
    println!("{}", j.pretty());
    println!("compact: {}", j);
    println!("get scores.math = {:?}", j.get("scores.math"));
    println!("get extra.5 = {:?}", j.get("extra.5").map(|x| x.to_string()));
    println!("get nope.x = {:?}", j.get("nope.x"));
    let p = Person::from_json(&j).expect("person");
    println!("{:?}", p.address);
    let mut keys: Vec<_> = p.scores.keys().cloned().collect();
    keys.sort();
    println!("{} {} {:?} {:?} {:?}", p.name, p.age, p.emails, keys, p.pos);
    println!("roundtrip person: {}", roundtrip(&p));

    let shapes = vec![Shape::Circle { r: 1.5 }, Shape::Rect { w: 2.0, h: 3.25 }, Shape::Unit];
    println!("{:#}", shapes.to_json());
    println!("roundtrip shapes: {}", roundtrip(&shapes));
    let nested: Vec<Option<(String, Vec<u32>)>> = vec![Some(("a".into(), vec![1, 2, 3])), None, Some(("".into(), vec![]))];
    println!("nested: {} rt={}", nested.to_json(), roundtrip(&nested));
    let words: &[&str] = &["x", "y\"z"];
    println!("slice: {}", words.to_json());

    let bad_docs = [
        "",
        "[1, 2,",
        "{\"a\" 1}",
        "{\"a\": 1, \"a\": 2}",
        "[1 2]",
        "\"abc",
        "tru",
        "{\"k\": \"\\x\"}",
        "[1]x",
        "\n\n  [\n   @]",
        "-",
        "\"\\uZZZZ\"",
    ];
    for b in bad_docs.iter() {
        match parse(b) {
            Ok(v) => println!("{:?} => ok {}", b, v),
            Err(e) => println!("{:?} => {}", b, e),
        }
    }
    let type_errors = [
        r#"{"name": "x", "age": 300, "emails": [], "scores": {}, "pos": [0,0]}"#,
        r#"{"name": "x", "age": 3, "emails": ["a", 5], "scores": {}, "pos": [0,0]}"#,
        r#"{"name": "x", "age": 3, "emails": [], "scores": {"m": "high"}, "pos": [0,0]}"#,
        r#"{"age": 3, "emails": [], "scores": {}, "pos": [0,0]}"#,
        r#"{"name": "x", "age": 3.5, "emails": [], "scores": {}, "pos": [0]}"#,
        r#"[{"type": "circle"}]"#,
        r#"[{"type": "hexagon"}]"#,
    ];
    for t in type_errors.iter() {
        let j = parse(t).unwrap();
        let r: Result<Box<dyn fmt::Debug>, DeError> = if t.starts_with('[') {
            Vec::<Shape>::from_json(&j).map(|v| Box::new(v) as Box<dyn fmt::Debug>)
        } else {
            Person::from_json(&j).map(|v| Box::new(v.age) as Box<dyn fmt::Debug>)
        };
        match r {
            Ok(v) => println!("ok {:?}", v),
            Err(e) => println!("error: {}", e),
        }
    }
    // big generated doc
    let mut big = Vec::new();
    for i in 0..200u32 {
        let mut m = HashMap::new();
        m.insert(format!("k{}", i % 7), (i as f64) / 8.0);
        big.push((format!("item-{:03}", i), m));
    }
    let text = big.to_json().to_string();
    let parsed = parse(&text).unwrap();
    let back: Vec<(String, HashMap<String, f64>)> = Deserialize::from_json(&parsed).unwrap();
    let total: f64 = back.iter().flat_map(|(_, m)| m.values()).sum();
    println!("big: {} bytes, {} items, total {}, eq {}", text.len(), back.len(), total, back == big);
}
