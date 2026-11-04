// JSON parser + pretty printer + a small path query language.
use std::collections::BTreeMap;
use std::fmt::{self, Write as _};

#[derive(Debug, Clone, PartialEq)]
enum Json {
    Null,
    Bool(bool),
    Num(f64),
    Str(String),
    Arr(Vec<Json>),
    Obj(BTreeMap<String, Json>),
}

#[derive(Debug)]
struct ParseError {
    pos: usize,
    msg: String,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "parse error at byte {}: {}", self.pos, self.msg)
    }
}

impl std::error::Error for ParseError {}

struct JsonParser<'a> {
    b: &'a [u8],
    i: usize,
    depth: usize,
}

impl<'a> JsonParser<'a> {
    fn parse(s: &'a str) -> Result<Json, ParseError> {
        let mut p = JsonParser { b: s.as_bytes(), i: 0, depth: 0 };
        let v = p.value()?;
        p.ws();
        if p.i != p.b.len() {
            return Err(p.err("trailing characters"));
        }
        Ok(v)
    }

    fn err(&self, m: &str) -> ParseError {
        ParseError { pos: self.i, msg: m.to_string() }
    }

    fn ws(&mut self) {
        while self.i < self.b.len() && matches!(self.b[self.i], b' ' | b'\n' | b'\t' | b'\r') {
            self.i += 1;
        }
    }

    fn lit(&mut self, s: &str, v: Json) -> Result<Json, ParseError> {
        if self.b[self.i..].starts_with(s.as_bytes()) {
            self.i += s.len();
            Ok(v)
        } else {
            Err(self.err(&format!("expected '{}'", s)))
        }
    }

    fn value(&mut self) -> Result<Json, ParseError> {
        self.ws();
        if self.i >= self.b.len() {
            return Err(self.err("unexpected end of input"));
        }
        match self.b[self.i] {
            b'n' => self.lit("null", Json::Null),
            b't' => self.lit("true", Json::Bool(true)),
            b'f' => self.lit("false", Json::Bool(false)),
            b'"' => Ok(Json::Str(self.string()?)),
            b'[' => {
                self.depth += 1;
                if self.depth > 64 {
                    return Err(self.err("nesting too deep"));
                }
                self.i += 1;
                let mut v = Vec::new();
                self.ws();
                if self.peek() == Some(b']') {
                    self.i += 1;
                    self.depth -= 1;
                    return Ok(Json::Arr(v));
                }
                loop {
                    v.push(self.value()?);
                    self.ws();
                    match self.peek() {
                        Some(b',') => self.i += 1,
                        Some(b']') => {
                            self.i += 1;
                            break;
                        }
                        _ => return Err(self.err("expected ',' or ']'")),
                    }
                }
                self.depth -= 1;
                Ok(Json::Arr(v))
            }
            b'{' => {
                self.i += 1;
                let mut m = BTreeMap::new();
                self.ws();
                if self.peek() == Some(b'}') {
                    self.i += 1;
                    return Ok(Json::Obj(m));
                }
                loop {
                    self.ws();
                    if self.peek() != Some(b'"') {
                        return Err(self.err("expected string key"));
                    }
                    let k = self.string()?;
                    self.ws();
                    if self.peek() != Some(b':') {
                        return Err(self.err("expected ':'"));
                    }
                    self.i += 1;
                    let v = self.value()?;
                    m.insert(k, v);
                    self.ws();
                    match self.peek() {
                        Some(b',') => self.i += 1,
                        Some(b'}') => {
                            self.i += 1;
                            break;
                        }
                        _ => return Err(self.err("expected ',' or '}'")),
                    }
                }
                Ok(Json::Obj(m))
            }
            b'-' | b'0'..=b'9' => self.number(),
            c => Err(self.err(&format!("unexpected character {:?}", c as char))),
        }
    }

    fn peek(&self) -> Option<u8> {
        self.b.get(self.i).copied()
    }

    fn number(&mut self) -> Result<Json, ParseError> {
        let start = self.i;
        if self.peek() == Some(b'-') {
            self.i += 1;
        }
        while let Some(c) = self.peek() {
            if c.is_ascii_digit() || matches!(c, b'.' | b'e' | b'E' | b'+' | b'-') {
                self.i += 1;
            } else {
                break;
            }
        }
        let s = std::str::from_utf8(&self.b[start..self.i]).unwrap();
        s.parse::<f64>().map(Json::Num).map_err(|_| ParseError { pos: start, msg: format!("invalid number {:?}", s) })
    }

    fn hex4(&mut self) -> Result<u32, ParseError> {
        if self.i + 4 > self.b.len() {
            return Err(self.err("short unicode escape"));
        }
        let s = std::str::from_utf8(&self.b[self.i..self.i + 4]).map_err(|_| self.err("bad utf8"))?;
        let v = u32::from_str_radix(s, 16).map_err(|_| self.err("bad hex"))?;
        self.i += 4;
        Ok(v)
    }

    fn string(&mut self) -> Result<String, ParseError> {
        self.i += 1; // opening quote
        let mut out = String::new();
        loop {
            let c = match self.peek() {
                None => return Err(self.err("unterminated string")),
                Some(c) => c,
            };
            match c {
                b'"' => {
                    self.i += 1;
                    return Ok(out);
                }
                b'\\' => {
                    self.i += 1;
                    let e = self.peek().ok_or_else(|| self.err("bad escape"))?;
                    self.i += 1;
                    match e {
                        b'n' => out.push('\n'),
                        b't' => out.push('\t'),
                        b'r' => out.push('\r'),
                        b'b' => out.push('\u{8}'),
                        b'f' => out.push('\u{c}'),
                        b'/' => out.push('/'),
                        b'\\' => out.push('\\'),
                        b'"' => out.push('"'),
                        b'u' => {
                            let hi = self.hex4()?;
                            let cp = if (0xD800..0xDC00).contains(&hi) {
                                if !self.b[self.i..].starts_with(b"\\u") {
                                    return Err(self.err("lone surrogate"));
                                }
                                self.i += 2;
                                let lo = self.hex4()?;
                                0x10000 + ((hi - 0xD800) << 10) + (lo - 0xDC00)
                            } else {
                                hi
                            };
                            out.push(char::from_u32(cp).ok_or_else(|| self.err("invalid code point"))?);
                        }
                        _ => return Err(self.err("unknown escape")),
                    }
                }
                _ => {
                    // copy a whole UTF-8 sequence
                    let len = match c {
                        0x00..=0x7f => 1,
                        0xc0..=0xdf => 2,
                        0xe0..=0xef => 3,
                        _ => 4,
                    };
                    out.push_str(std::str::from_utf8(&self.b[self.i..self.i + len]).unwrap());
                    self.i += len;
                }
            }
        }
    }
}

fn escape(s: &str, out: &mut String) {
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            '\r' => out.push_str("\\r"),
            c if (c as u32) < 0x20 => {
                write!(out, "\\u{:04x}", c as u32).unwrap();
            }
            c => out.push(c),
        }
    }
    out.push('"');
}

fn fmt_num(n: f64) -> String {
    if n.fract() == 0.0 && n.abs() < 1e16 {
        format!("{}", n as i64)
    } else {
        format!("{}", n)
    }
}

impl Json {
    fn compact(&self) -> String {
        let mut s = String::new();
        self.write_compact(&mut s);
        s
    }
    fn write_compact(&self, out: &mut String) {
        match self {
            Json::Null => out.push_str("null"),
            Json::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
            Json::Num(n) => out.push_str(&fmt_num(*n)),
            Json::Str(s) => escape(s, out),
            Json::Arr(v) => {
                out.push('[');
                for (i, x) in v.iter().enumerate() {
                    if i > 0 {
                        out.push(',');
                    }
                    x.write_compact(out);
                }
                out.push(']');
            }
            Json::Obj(m) => {
                out.push('{');
                for (i, (k, v)) in m.iter().enumerate() {
                    if i > 0 {
                        out.push(',');
                    }
                    escape(k, out);
                    out.push(':');
                    v.write_compact(out);
                }
                out.push('}');
            }
        }
    }
    fn pretty(&self, indent: usize, out: &mut String) {
        let pad = "  ".repeat(indent);
        let pad1 = "  ".repeat(indent + 1);
        match self {
            Json::Arr(v) if !v.is_empty() => {
                if v.iter().all(|x| !matches!(x, Json::Arr(_) | Json::Obj(_))) && v.len() <= 6 {
                    self.write_compact(out);
                    return;
                }
                out.push_str("[\n");
                for (i, x) in v.iter().enumerate() {
                    out.push_str(&pad1);
                    x.pretty(indent + 1, out);
                    if i + 1 < v.len() {
                        out.push(',');
                    }
                    out.push('\n');
                }
                out.push_str(&pad);
                out.push(']');
            }
            Json::Obj(m) if !m.is_empty() => {
                out.push_str("{\n");
                let n = m.len();
                for (i, (k, v)) in m.iter().enumerate() {
                    out.push_str(&pad1);
                    escape(k, out);
                    out.push_str(": ");
                    v.pretty(indent + 1, out);
                    if i + 1 < n {
                        out.push(',');
                    }
                    out.push('\n');
                }
                out.push_str(&pad);
                out.push('}');
            }
            other => other.write_compact(out),
        }
    }
    fn kind(&self) -> &'static str {
        match self {
            Json::Null => "null",
            Json::Bool(_) => "bool",
            Json::Num(_) => "number",
            Json::Str(_) => "string",
            Json::Arr(_) => "array",
            Json::Obj(_) => "object",
        }
    }
}

// ---------------- query language ----------------
// path := segment*
// segment := '.' ident | '[' int ']' | '[*]' | '..' ident | '[?' field op literal ']'
#[derive(Debug, Clone)]
enum Seg {
    Key(String),
    Index(i64),
    Wild,
    Recursive(String),
    Filter(String, String, Json),
    Len,
}

fn parse_query(q: &str) -> Result<Vec<Seg>, String> {
    let mut segs = Vec::new();
    let cs: Vec<char> = q.chars().collect();
    let mut i = 0;
    let ident = |i: &mut usize| -> String {
        let st = *i;
        while *i < cs.len() && (cs[*i].is_alphanumeric() || cs[*i] == '_') {
            *i += 1;
        }
        cs[st..*i].iter().collect()
    };
    if cs.first() == Some(&'$') {
        i = 1;
    }
    while i < cs.len() {
        match cs[i] {
            '.' if cs.get(i + 1) == Some(&'.') => {
                i += 2;
                segs.push(Seg::Recursive(ident(&mut i)));
            }
            '.' => {
                i += 1;
                let id = ident(&mut i);
                if id.is_empty() {
                    return Err(format!("empty key at {}", i));
                }
                if id == "length" {
                    segs.push(Seg::Len);
                } else {
                    segs.push(Seg::Key(id));
                }
            }
            '[' => {
                let close = cs[i..].iter().position(|&c| c == ']').ok_or("unclosed [")? + i;
                let inner: String = cs[i + 1..close].iter().collect();
                i = close + 1;
                if inner == "*" {
                    segs.push(Seg::Wild);
                } else if let Some(f) = inner.strip_prefix('?') {
                    let ops = ["==", "!=", ">=", "<=", ">", "<"];
                    let op = ops.iter().find(|o| f.contains(*o)).ok_or("filter needs operator")?;
                    let (field, lit) = f.split_once(op).unwrap();
                    let lit = JsonParser::parse(lit.trim()).map_err(|e| e.to_string())?;
                    segs.push(Seg::Filter(field.trim().to_string(), op.to_string(), lit));
                } else {
                    segs.push(Seg::Index(inner.trim().parse().map_err(|_| format!("bad index {:?}", inner))?));
                }
            }
            c => return Err(format!("unexpected {:?} at {}", c, i)),
        }
    }
    Ok(segs)
}

fn cmp_json(a: &Json, op: &str, b: &Json) -> bool {
    match (a, b) {
        (Json::Num(x), Json::Num(y)) => match op {
            "==" => x == y,
            "!=" => x != y,
            ">" => x > y,
            "<" => x < y,
            ">=" => x >= y,
            "<=" => x <= y,
            _ => false,
        },
        (Json::Str(x), Json::Str(y)) => match op {
            "==" => x == y,
            "!=" => x != y,
            ">" => x > y,
            "<" => x < y,
            _ => false,
        },
        _ => match op {
            "==" => a == b,
            "!=" => a != b,
            _ => false,
        },
    }
}

fn collect_recursive<'a>(v: &'a Json, key: &str, out: &mut Vec<&'a Json>) {
    match v {
        Json::Obj(m) => {
            for (k, x) in m {
                if k == key {
                    out.push(x);
                }
                collect_recursive(x, key, out);
            }
        }
        Json::Arr(a) => {
            for x in a {
                collect_recursive(x, key, out);
            }
        }
        _ => {}
    }
}

fn query(root: &Json, q: &str) -> Result<Vec<Json>, String> {
    let segs = parse_query(q)?;
    let mut cur: Vec<&Json> = vec![root];
    let mut owned: Vec<Json> = Vec::new();
    for seg in &segs {
        let mut next: Vec<&Json> = Vec::new();
        let mut lens = Vec::new();
        for v in &cur {
            match (seg, v) {
                (Seg::Key(k), Json::Obj(m)) => next.extend(m.get(k)),
                (Seg::Index(i), Json::Arr(a)) => {
                    let idx = if *i < 0 { a.len() as i64 + i } else { *i };
                    if idx >= 0 {
                        next.extend(a.get(idx as usize));
                    }
                }
                (Seg::Wild, Json::Arr(a)) => next.extend(a.iter()),
                (Seg::Wild, Json::Obj(m)) => next.extend(m.values()),
                (Seg::Recursive(k), _) => collect_recursive(v, k, &mut next),
                (Seg::Filter(f, op, lit), Json::Arr(a)) => {
                    for x in a {
                        if let Json::Obj(m) = x {
                            if let Some(fv) = m.get(f) {
                                if cmp_json(fv, op, lit) {
                                    next.push(x);
                                }
                            }
                        }
                    }
                }
                (Seg::Len, Json::Arr(a)) => lens.push(a.len()),
                (Seg::Len, Json::Obj(m)) => lens.push(m.len()),
                (Seg::Len, Json::Str(s)) => lens.push(s.chars().count()),
                _ => {}
            }
        }
        if matches!(seg, Seg::Len) {
            owned = lens.into_iter().map(|n| Json::Num(n as f64)).collect();
            return Ok(owned);
        }
        cur = next;
    }
    owned.extend(cur.into_iter().cloned());
    Ok(owned)
}

const DOC: &str = r#"
{
  "store": {
    "name": "Ye Olde \"Shoppe\"",
    "open": true,
    "rating": 4.25,
    "tags": ["books", "music", "caf\u00e9", "\ud83d\ude00"],
    "books": [
      {"title": "Dune", "author": "Herbert", "price": 9.99, "year": 1965, "stock": 3},
      {"title": "Neuromancer", "author": "Gibson", "price": 7.5, "year": 1984, "stock": 0},
      {"title": "Hyperion", "author": "Simmons", "price": 12, "year": 1989, "stock": 7},
      {"title": "Snow Crash", "author": "Stephenson", "price": 10.25, "year": 1992, "stock": 2,
       "reviews": [{"by": "ann", "stars": 5}, {"by": "bo", "stars": 3}]}
    ],
    "address": {"street": "1 Main St\nSuite 2", "zip": "02134", "geo": [42.35, -71.06]},
    "manager": null
  },
  "version": 3,
  "exp": 1.5e3,
  "neg": -0.125,
  "empty": {"a": [], "b": {}}
}
"#;

fn main() {
    let doc = match JsonParser::parse(DOC) {
        Ok(d) => d,
        Err(e) => {
            println!("{}", e);
            return;
        }
    };
    let mut s = String::new();
    doc.pretty(0, &mut s);
    println!("{}", s);
    println!("compact length: {}", doc.compact().len());

    let reparsed = JsonParser::parse(&doc.compact()).expect("round trip");
    println!("round trip equal: {}", reparsed == doc);
    let mut s2 = String::new();
    reparsed.pretty(0, &mut s2);
    println!("pretty round trip equal: {}", JsonParser::parse(&s2).unwrap() == doc);

    let queries = [
        "$.store.name",
        "$.store.books[0].title",
        "$.store.books[-1].reviews[*].by",
        "$.store.books[*].price",
        "$.store.books[?price > 9].title",
        "$.store.books[?author == \"Gibson\"].year",
        "$.store.books[?stock != 0].title",
        "$..stars",
        "$..zip",
        "$.store.tags.length",
        "$.store.books.length",
        "$.store.tags[2]",
        "$.store.tags[3].length",
        "$.store.address.geo",
        "$.empty.*",
        "$.store.missing",
        "$.store.books[x]",
        "$.store.books[?price]",
    ];
    for q in queries.iter() {
        match query(&doc, q) {
            Ok(res) => {
                let parts: Vec<String> = res.iter().map(|j| j.compact()).collect();
                println!("{:<45} => [{}]", q, parts.join(", "));
            }
            Err(e) => println!("{:<45} => error: {}", q, e),
        }
    }

    // aggregate using queries
    let prices = query(&doc, "$.store.books[*].price").unwrap();
    let total: f64 = prices.iter().filter_map(|j| if let Json::Num(n) = j { Some(*n) } else { None }).sum();
    println!("total price = {:.2}, avg = {:.3}", total, total / prices.len() as f64);

    // type census
    fn census(j: &Json, counts: &mut BTreeMap<&'static str, usize>) {
        *counts.entry(j.kind()).or_insert(0) += 1;
        match j {
            Json::Arr(a) => a.iter().for_each(|x| census(x, counts)),
            Json::Obj(m) => m.values().for_each(|x| census(x, counts)),
            _ => {}
        }
    }
    let mut counts = BTreeMap::new();
    census(&doc, &mut counts);
    println!("census: {:?}", counts);

    let bad = [
        "",
        "[1, 2",
        "{\"a\" 1}",
        "[1,]",
        "tru",
        "\"abc",
        "{\"k\": \"\\x\"}",
        "[1] 2",
        "-",
        "\"\\ud800\"",
        "{1: 2}",
        &"[".repeat(70),
    ];
    for b in bad.iter() {
        match JsonParser::parse(b) {
            Ok(v) => println!("{:?} unexpectedly parsed as {}", b, v.compact()),
            Err(e) => println!("{:<14} -> {}", format!("{:.12}", format!("{:?}", b)), e),
        }
    }

    let tricky = JsonParser::parse(r#"["\u0001\u001f", "tab\there", 0.1, 1e-7, 123456789012, -0, 1E2]"#).unwrap();
    println!("{}", tricky.compact());
    if let Json::Arr(a) = &tricky {
        for x in a {
            print!("{:?} ", x);
        }
        println!();
    }
}
