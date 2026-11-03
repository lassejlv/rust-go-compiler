// JSON parser + serializer with a generic visitor trait (serde-like design).
use std::collections::BTreeMap;
use std::fmt::Write as _;

#[derive(Debug, Clone, PartialEq)]
enum Json {
    Null,
    Bool(bool),
    Num(f64),
    Str(String),
    Arr(Vec<Json>),
    Obj(BTreeMap<String, Json>),
}

trait Visitor {
    type Output;
    fn visit_null(&mut self) -> Self::Output;
    fn visit_bool(&mut self, b: bool) -> Self::Output;
    fn visit_num(&mut self, n: f64) -> Self::Output;
    fn visit_str(&mut self, s: &str) -> Self::Output;
    fn visit_arr(&mut self, items: &[Json]) -> Self::Output;
    fn visit_obj(&mut self, m: &BTreeMap<String, Json>) -> Self::Output;
}

impl Json {
    fn accept<V: Visitor>(&self, v: &mut V) -> V::Output {
        match self {
            Json::Null => v.visit_null(),
            Json::Bool(b) => v.visit_bool(*b),
            Json::Num(n) => v.visit_num(*n),
            Json::Str(s) => v.visit_str(s),
            Json::Arr(a) => v.visit_arr(a),
            Json::Obj(m) => v.visit_obj(m),
        }
    }
}

struct Compact;
impl Visitor for Compact {
    type Output = String;
    fn visit_null(&mut self) -> String { "null".into() }
    fn visit_bool(&mut self, b: bool) -> String { b.to_string() }
    fn visit_num(&mut self, n: f64) -> String {
        if n.fract() == 0.0 && n.abs() < 1e15 { format!("{}", n as i64) } else { format!("{}", n) }
    }
    fn visit_str(&mut self, s: &str) -> String { escape(s) }
    fn visit_arr(&mut self, items: &[Json]) -> String {
        let parts: Vec<String> = items.iter().map(|j| j.accept(self)).collect();
        format!("[{}]", parts.join(","))
    }
    fn visit_obj(&mut self, m: &BTreeMap<String, Json>) -> String {
        let parts: Vec<String> = m.iter().map(|(k, v)| format!("{}:{}", escape(k), v.accept(self))).collect();
        format!("{{{}}}", parts.join(","))
    }
}

struct Pretty { depth: usize, out: String }
impl Visitor for Pretty {
    type Output = ();
    fn visit_null(&mut self) { self.out.push_str("null") }
    fn visit_bool(&mut self, b: bool) { write!(self.out, "{}", b).unwrap() }
    fn visit_num(&mut self, n: f64) { let s = Compact.visit_num(n); self.out.push_str(&s) }
    fn visit_str(&mut self, s: &str) { self.out.push_str(&escape(s)) }
    fn visit_arr(&mut self, items: &[Json]) {
        if items.is_empty() { self.out.push_str("[]"); return; }
        self.out.push_str("[\n");
        self.depth += 1;
        for (i, it) in items.iter().enumerate() {
            self.out.push_str(&"  ".repeat(self.depth));
            it.accept(self);
            if i + 1 < items.len() { self.out.push(','); }
            self.out.push('\n');
        }
        self.depth -= 1;
        let _ = write!(self.out, "{}]", "  ".repeat(self.depth));
    }
    fn visit_obj(&mut self, m: &BTreeMap<String, Json>) {
        if m.is_empty() { self.out.push_str("{}"); return; }
        self.out.push_str("{\n");
        self.depth += 1;
        let n = m.len();
        for (i, (k, v)) in m.iter().enumerate() {
            let _ = write!(self.out, "{}{}: ", "  ".repeat(self.depth), escape(k));
            v.accept(self);
            self.out.push_str(if i + 1 < n { ",\n" } else { "\n" });
        }
        self.depth -= 1;
        let _ = write!(self.out, "{}}}", "  ".repeat(self.depth));
    }
}

#[derive(Default, Debug)]
struct Stats { nodes: usize, max_depth: usize, cur: usize, strings: usize, sum: f64 }
impl Visitor for Stats {
    type Output = ();
    fn visit_null(&mut self) { self.nodes += 1 }
    fn visit_bool(&mut self, _: bool) { self.nodes += 1 }
    fn visit_num(&mut self, n: f64) { self.nodes += 1; self.sum += n }
    fn visit_str(&mut self, _: &str) { self.nodes += 1; self.strings += 1 }
    fn visit_arr(&mut self, items: &[Json]) {
        self.nodes += 1;
        self.cur += 1;
        self.max_depth = self.max_depth.max(self.cur);
        items.iter().for_each(|i| i.accept(self));
        self.cur -= 1;
    }
    fn visit_obj(&mut self, m: &BTreeMap<String, Json>) {
        self.nodes += 1;
        self.cur += 1;
        self.max_depth = self.max_depth.max(self.cur);
        for v in m.values() { v.accept(self) }
        self.cur -= 1;
    }
}

fn escape(s: &str) -> String {
    let mut o = String::with_capacity(s.len() + 2);
    o.push('"');
    for c in s.chars() {
        match c {
            '"' => o.push_str("\\\""),
            '\\' => o.push_str("\\\\"),
            '\n' => o.push_str("\\n"),
            '\t' => o.push_str("\\t"),
            c if (c as u32) < 0x20 => { let _ = write!(o, "\\u{:04x}", c as u32); }
            c => o.push(c),
        }
    }
    o.push('"');
    o
}

struct P<'a> { s: &'a [u8], i: usize }

impl<'a> P<'a> {
    fn ws(&mut self) { while self.i < self.s.len() && (self.s[self.i] as char).is_ascii_whitespace() { self.i += 1 } }
    fn err<T>(&self, m: &str) -> Result<T, String> { Err(format!("{} at byte {}", m, self.i)) }
    fn value(&mut self) -> Result<Json, String> {
        self.ws();
        match self.s.get(self.i) {
            None => self.err("eof"),
            Some(b'n') => self.lit("null", Json::Null),
            Some(b't') => self.lit("true", Json::Bool(true)),
            Some(b'f') => self.lit("false", Json::Bool(false)),
            Some(b'"') => Ok(Json::Str(self.string()?)),
            Some(b'[') => {
                self.i += 1;
                let mut v = Vec::new();
                self.ws();
                if self.s.get(self.i) == Some(&b']') { self.i += 1; return Ok(Json::Arr(v)); }
                loop {
                    v.push(self.value()?);
                    self.ws();
                    match self.s.get(self.i) {
                        Some(b',') => self.i += 1,
                        Some(b']') => { self.i += 1; return Ok(Json::Arr(v)); }
                        _ => return self.err("expected , or ]"),
                    }
                }
            }
            Some(b'{') => {
                self.i += 1;
                let mut m = BTreeMap::new();
                self.ws();
                if self.s.get(self.i) == Some(&b'}') { self.i += 1; return Ok(Json::Obj(m)); }
                loop {
                    self.ws();
                    if self.s.get(self.i) != Some(&b'"') { return self.err("expected key"); }
                    let k = self.string()?;
                    self.ws();
                    if self.s.get(self.i) != Some(&b':') { return self.err("expected :"); }
                    self.i += 1;
                    let v = self.value()?;
                    m.insert(k, v);
                    self.ws();
                    match self.s.get(self.i) {
                        Some(b',') => self.i += 1,
                        Some(b'}') => { self.i += 1; return Ok(Json::Obj(m)); }
                        _ => return self.err("expected , or }"),
                    }
                }
            }
            Some(c) if *c == b'-' || c.is_ascii_digit() => {
                let st = self.i;
                self.i += 1;
                while self.i < self.s.len() && matches!(self.s[self.i], b'0'..=b'9' | b'.' | b'e' | b'E' | b'+' | b'-') { self.i += 1 }
                let t = std::str::from_utf8(&self.s[st..self.i]).unwrap();
                t.parse::<f64>().map(Json::Num).map_err(|e| format!("bad number {:?}: {}", t, e))
            }
            Some(_) => self.err("unexpected character"),
        }
    }
    fn lit(&mut self, w: &str, j: Json) -> Result<Json, String> {
        if self.s[self.i..].starts_with(w.as_bytes()) { self.i += w.len(); Ok(j) } else { self.err("bad literal") }
    }
    fn string(&mut self) -> Result<String, String> {
        self.i += 1;
        let mut out = Vec::new();
        loop {
            match self.s.get(self.i).copied() {
                None => return self.err("unterminated string"),
                Some(b'"') => { self.i += 1; break; }
                Some(b'\\') => {
                    self.i += 1;
                    let c = self.s.get(self.i).copied().ok_or("bad escape")?;
                    match c {
                        b'n' => out.push(b'\n'),
                        b't' => out.push(b'\t'),
                        b'u' => {
                            let h = std::str::from_utf8(&self.s[self.i + 1..self.i + 5]).unwrap();
                            let cp = u32::from_str_radix(h, 16).map_err(|e| e.to_string())?;
                            let ch = char::from_u32(cp).ok_or("bad codepoint")?;
                            let mut buf = [0u8; 4];
                            out.extend_from_slice(ch.encode_utf8(&mut buf).as_bytes());
                            self.i += 4;
                        }
                        other => out.push(other),
                    }
                    self.i += 1;
                }
                Some(b) => { out.push(b); self.i += 1; }
            }
        }
        String::from_utf8(out).map_err(|e| e.to_string())
    }
}

fn parse(s: &str) -> Result<Json, String> {
    let mut p = P { s: s.as_bytes(), i: 0 };
    let v = p.value()?;
    p.ws();
    if p.i != s.len() { return p.err("trailing data"); }
    Ok(v)
}

fn main() {
    let src = r#" { "name": "rustgo", "version": 1.5, "tags": ["compiler", "go", "été"],
        "deps": {"core": {"ok": true, "n": -3}, "empty": {}, "list": []}, "note": "line1\nline2 \"q\"", "nil": null } "#;
    let j = parse(src).unwrap();
    println!("{}", j.accept(&mut Compact));
    let mut p = Pretty { depth: 0, out: String::new() };
    j.accept(&mut p);
    println!("{}", p.out);
    let mut st = Stats::default();
    j.accept(&mut st);
    println!("{:?}", st);
    let again = parse(&j.accept(&mut Compact)).unwrap();
    println!("roundtrip equal: {}", again == j);
    for bad in ["[1,2", "{\"a\" 1}", "tru", "[1,]x", "\"abc", "1.2.3", "{} x"] {
        println!("{:<10} => {:?}", bad, parse(bad));
    }
    if let Json::Obj(m) = &j {
        let keys: Vec<&String> = m.keys().collect();
        println!("keys {:?}", keys);
        if let Some(Json::Arr(tags)) = m.get("tags") {
            println!("tags[2] = {:?}", tags[2]);
        }
    }
}
