// A serde_json-like Value enum with a byte-level recursive descent parser (escapes, \u surrogates,
// numbers), a pretty printer, pointer lookup ("/a/0/b"), Index impls, and From conversions.
use std::collections::BTreeMap;
use std::fmt::{self, Write};
use std::ops::Index;

#[derive(Debug, Clone, PartialEq)]
enum Value {
    Null,
    Bool(bool),
    Number(f64),
    String(String),
    Array(Vec<Value>),
    Object(BTreeMap<String, Value>),
}

#[derive(Debug)]
struct ParseError {
    msg: String,
    pos: usize,
}

struct P<'a> {
    b: &'a [u8],
    i: usize,
}

impl<'a> P<'a> {
    fn err<T>(&self, msg: impl Into<String>) -> Result<T, ParseError> {
        Err(ParseError { msg: msg.into(), pos: self.i })
    }
    fn ws(&mut self) {
        while self.i < self.b.len() && matches!(self.b[self.i], b' ' | b'\n' | b'\t' | b'\r') {
            self.i += 1;
        }
    }
    fn eat(&mut self, c: u8) -> Result<(), ParseError> {
        self.ws();
        if self.b.get(self.i) == Some(&c) {
            self.i += 1;
            Ok(())
        } else {
            self.err(format!("expected '{}'", c as char))
        }
    }
    fn value(&mut self) -> Result<Value, ParseError> {
        self.ws();
        match self.b.get(self.i) {
            None => self.err("unexpected end"),
            Some(b'n') => self.lit("null", Value::Null),
            Some(b't') => self.lit("true", Value::Bool(true)),
            Some(b'f') => self.lit("false", Value::Bool(false)),
            Some(b'"') => Ok(Value::String(self.string()?)),
            Some(b'[') => {
                self.i += 1;
                let mut v = vec![];
                self.ws();
                if self.b.get(self.i) == Some(&b']') {
                    self.i += 1;
                    return Ok(Value::Array(v));
                }
                loop {
                    v.push(self.value()?);
                    self.ws();
                    match self.b.get(self.i) {
                        Some(b',') => self.i += 1,
                        Some(b']') => {
                            self.i += 1;
                            return Ok(Value::Array(v));
                        }
                        _ => return self.err("expected ',' or ']'"),
                    }
                }
            }
            Some(b'{') => {
                self.i += 1;
                let mut m = BTreeMap::new();
                self.ws();
                if self.b.get(self.i) == Some(&b'}') {
                    self.i += 1;
                    return Ok(Value::Object(m));
                }
                loop {
                    self.ws();
                    let k = self.string()?;
                    self.eat(b':')?;
                    let v = self.value()?;
                    m.insert(k, v);
                    self.ws();
                    match self.b.get(self.i) {
                        Some(b',') => self.i += 1,
                        Some(b'}') => {
                            self.i += 1;
                            return Ok(Value::Object(m));
                        }
                        _ => return self.err("expected ',' or '}'"),
                    }
                }
            }
            Some(c) if *c == b'-' || c.is_ascii_digit() => self.number(),
            Some(c) => self.err(format!("unexpected '{}'", *c as char)),
        }
    }
    fn lit(&mut self, s: &str, v: Value) -> Result<Value, ParseError> {
        if self.b[self.i..].starts_with(s.as_bytes()) {
            self.i += s.len();
            Ok(v)
        } else {
            self.err(format!("expected {}", s))
        }
    }
    fn number(&mut self) -> Result<Value, ParseError> {
        let st = self.i;
        if self.b[self.i] == b'-' {
            self.i += 1;
        }
        while self.i < self.b.len() && matches!(self.b[self.i], b'0'..=b'9' | b'.' | b'e' | b'E' | b'+' | b'-') {
            self.i += 1;
        }
        let s = std::str::from_utf8(&self.b[st..self.i]).unwrap();
        s.parse::<f64>().map(Value::Number).or_else(|_| self.err(format!("bad number {:?}", s)))
    }
    fn hex4(&mut self) -> Result<u32, ParseError> {
        let h = self.b.get(self.i..self.i + 4).and_then(|h| std::str::from_utf8(h).ok()).and_then(|h| u32::from_str_radix(h, 16).ok());
        match h {
            Some(v) => {
                self.i += 4;
                Ok(v)
            }
            None => self.err("bad \\u escape"),
        }
    }
    fn string(&mut self) -> Result<String, ParseError> {
        if self.b.get(self.i) != Some(&b'"') {
            return self.err("expected string");
        }
        self.i += 1;
        let mut out = Vec::new();
        loop {
            let Some(&c) = self.b.get(self.i) else { return self.err("unterminated string") };
            self.i += 1;
            match c {
                b'"' => break,
                b'\\' => {
                    let e = self.b.get(self.i).copied();
                    self.i += 1;
                    let ch = match e {
                        Some(b'n') => '\n',
                        Some(b't') => '\t',
                        Some(b'"') => '"',
                        Some(b'\\') => '\\',
                        Some(b'/') => '/',
                        Some(b'u') => {
                            let hi = self.hex4()?;
                            let cp = if (0xD800..0xDC00).contains(&hi) {
                                if self.b.get(self.i..self.i + 2) != Some(b"\\u") {
                                    return self.err("lone surrogate");
                                }
                                self.i += 2;
                                let lo = self.hex4()?;
                                0x10000 + ((hi - 0xD800) << 10) + (lo - 0xDC00)
                            } else {
                                hi
                            };
                            char::from_u32(cp).ok_or(ParseError { msg: "bad codepoint".into(), pos: self.i })?
                        }
                        _ => return self.err("bad escape"),
                    };
                    let mut buf = [0; 4];
                    out.extend_from_slice(ch.encode_utf8(&mut buf).as_bytes());
                }
                c => out.push(c),
            }
        }
        String::from_utf8(out).or_else(|_| self.err("invalid utf8"))
    }
}

fn parse(s: &str) -> Result<Value, ParseError> {
    let mut p = P { b: s.as_bytes(), i: 0 };
    let v = p.value()?;
    p.ws();
    if p.i != p.b.len() {
        return p.err("trailing characters");
    }
    Ok(v)
}

fn write_str(out: &mut String, s: &str) {
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

impl Value {
    fn write(&self, out: &mut String, indent: Option<usize>, level: usize) {
        let nl = |out: &mut String, l: usize| {
            if let Some(n) = indent {
                out.push('\n');
                out.push_str(&" ".repeat(n * l));
            }
        };
        match self {
            Value::Null => out.push_str("null"),
            Value::Bool(b) => write!(out, "{}", b).unwrap(),
            Value::Number(n) => write!(out, "{}", n).unwrap(),
            Value::String(s) => write_str(out, s),
            Value::Array(a) if a.is_empty() => out.push_str("[]"),
            Value::Object(o) if o.is_empty() => out.push_str("{}"),
            Value::Array(a) => {
                out.push('[');
                for (i, v) in a.iter().enumerate() {
                    if i > 0 {
                        out.push(',');
                    }
                    nl(out, level + 1);
                    v.write(out, indent, level + 1);
                }
                nl(out, level);
                out.push(']');
            }
            Value::Object(o) => {
                out.push('{');
                for (i, (k, v)) in o.iter().enumerate() {
                    if i > 0 {
                        out.push(',');
                    }
                    nl(out, level + 1);
                    write_str(out, k);
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
    fn pointer(&self, p: &str) -> Option<&Value> {
        if p.is_empty() {
            return Some(self);
        }
        p.strip_prefix('/')?.split('/').try_fold(self, |v, tok| match v {
            Value::Object(o) => o.get(tok),
            Value::Array(a) => a.get(tok.parse::<usize>().ok()?),
            _ => None,
        })
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut s = String::new();
        self.write(&mut s, if f.alternate() { Some(2) } else { None }, 0);
        f.write_str(&s)
    }
}

static NULL: Value = Value::Null;
impl Index<&str> for Value {
    type Output = Value;
    fn index(&self, k: &str) -> &Value {
        match self {
            Value::Object(o) => o.get(k).unwrap_or(&NULL),
            _ => &NULL,
        }
    }
}
impl Index<usize> for Value {
    type Output = Value;
    fn index(&self, i: usize) -> &Value {
        match self {
            Value::Array(a) => a.get(i).unwrap_or(&NULL),
            _ => &NULL,
        }
    }
}

impl From<i32> for Value {
    fn from(n: i32) -> Self {
        Value::Number(n as f64)
    }
}
impl From<&str> for Value {
    fn from(s: &str) -> Self {
        Value::String(s.into())
    }
}
impl<T: Into<Value>> From<Vec<T>> for Value {
    fn from(v: Vec<T>) -> Self {
        Value::Array(v.into_iter().map(Into::into).collect())
    }
}

fn main() {
    let src = r#"
    {
        "name": "zed",
        "version": [0, 150, 2],
        "features": {"vim": true, "copilot": false, "ratio": -1.25e2},
        "authors": [{"name": "A\u00e9", "emoji": "\ud83e\udd80"}, {"name": "B \"quoted\"\n"}],
        "empty": {}, "list": [], "nothing": null
    }"#;
    let v = parse(src).unwrap();
    println!("{}", v);
    println!("{:#}", v);
    println!("{} {} {}", v["name"], v["version"][1], v["features"]["ratio"]);
    println!("{} {}", v["missing"]["deep"], v["version"][99]);
    for p in ["/authors/0/emoji", "/authors/1/name", "/version/2", "/features", "/nope", "", "/version/x"] {
        println!("{:<18} -> {:?}", p, v.pointer(p).map(|x| x.to_string()));
    }
    let built = Value::from(vec![Value::from(1), Value::from("two"), Value::from(vec![3, 4])]);
    println!("{} eq-roundtrip={}", built, parse(&built.to_string()).unwrap() == built);
    for bad in ["[1, 2", "{\"a\" 1}", "tru", "\"abc", "[1,]", "{} x", "\"\\ud800\"", "-"] {
        let e = parse(bad).unwrap_err();
        println!("{:<12} -> {} at {}", bad, e.msg, e.pos);
    }
}
