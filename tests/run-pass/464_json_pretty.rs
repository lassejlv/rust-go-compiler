// A JSON-ish value type with a hand-written parser (strings with escapes incl. \uXXXX, numbers with
// exponents, nested arrays/objects preserving insertion order), a pretty printer with indentation,
// a compact printer, path queries, and structural equality.
use std::fmt::Write;

#[derive(Debug, Clone, PartialEq)]
enum Json {
    Null,
    Bool(bool),
    Num(f64),
    Str(String),
    Arr(Vec<Json>),
    Obj(Vec<(String, Json)>),
}

struct P<'a> {
    s: &'a [u8],
    i: usize,
}

impl<'a> P<'a> {
    fn ws(&mut self) {
        while self.i < self.s.len() && (self.s[self.i] as char).is_ascii_whitespace() { self.i += 1; }
    }
    fn err<T>(&self, m: &str) -> Result<T, String> {
        Err(format!("{} at {}", m, self.i))
    }
    fn lit(&mut self, w: &str, v: Json) -> Result<Json, String> {
        if self.s[self.i..].starts_with(w.as_bytes()) { self.i += w.len(); Ok(v) } else { self.err("bad literal") }
    }
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
                let mut v = vec![];
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
                let mut v = vec![];
                self.ws();
                if self.s.get(self.i) == Some(&b'}') { self.i += 1; return Ok(Json::Obj(v)); }
                loop {
                    self.ws();
                    let k = self.string()?;
                    self.ws();
                    if self.s.get(self.i) != Some(&b':') { return self.err("expected :"); }
                    self.i += 1;
                    let val = self.value()?;
                    v.push((k, val));
                    self.ws();
                    match self.s.get(self.i) {
                        Some(b',') => self.i += 1,
                        Some(b'}') => { self.i += 1; return Ok(Json::Obj(v)); }
                        _ => return self.err("expected , or }"),
                    }
                }
            }
            Some(c) if *c == b'-' || c.is_ascii_digit() => {
                let st = self.i;
                self.i += 1;
                while self.i < self.s.len() && matches!(self.s[self.i], b'0'..=b'9' | b'.' | b'e' | b'E' | b'+' | b'-') {
                    self.i += 1;
                }
                let t = std::str::from_utf8(&self.s[st..self.i]).unwrap();
                t.parse::<f64>().map(Json::Num).map_err(|_| format!("bad number {:?}", t))
            }
            Some(_) => self.err("unexpected byte"),
        }
    }
    fn string(&mut self) -> Result<String, String> {
        if self.s.get(self.i) != Some(&b'"') { return self.err("expected string"); }
        self.i += 1;
        let mut out = Vec::new();
        loop {
            match self.s.get(self.i) {
                None => return self.err("unterminated string"),
                Some(b'"') => { self.i += 1; return String::from_utf8(out).map_err(|e| e.to_string()); }
                Some(b'\\') => {
                    self.i += 1;
                    let c = match self.s.get(self.i) {
                        Some(b'n') => '\n',
                        Some(b't') => '\t',
                        Some(b'"') => '"',
                        Some(b'\\') => '\\',
                        Some(b'/') => '/',
                        Some(b'u') => {
                            let h = std::str::from_utf8(&self.s[self.i + 1..self.i + 5]).map_err(|e| e.to_string())?;
                            self.i += 4;
                            char::from_u32(u32::from_str_radix(h, 16).map_err(|e| e.to_string())?).unwrap_or('?')
                        }
                        _ => return self.err("bad escape"),
                    };
                    let mut b = [0; 4];
                    out.extend_from_slice(c.encode_utf8(&mut b).as_bytes());
                    self.i += 1;
                }
                Some(&b) => { out.push(b); self.i += 1; }
            }
        }
    }
}

fn parse(s: &str) -> Result<Json, String> {
    let mut p = P { s: s.as_bytes(), i: 0 };
    let v = p.value()?;
    p.ws();
    if p.i != s.len() { return p.err("trailing data"); }
    Ok(v)
}

fn num(n: f64) -> String {
    if n.fract() == 0.0 && n.abs() < 1e15 { format!("{}", n as i64) } else { format!("{}", n) }
}

fn pretty(j: &Json, ind: usize, out: &mut String) {
    let pad = "  ".repeat(ind);
    match j {
        Json::Null => out.push_str("null"),
        Json::Bool(b) => write!(out, "{}", b).unwrap(),
        Json::Num(n) => out.push_str(&num(*n)),
        Json::Str(s) => write!(out, "{:?}", s).unwrap(),
        Json::Arr(v) if v.is_empty() => out.push_str("[]"),
        Json::Obj(v) if v.is_empty() => out.push_str("{}"),
        Json::Arr(v) => {
            out.push_str("[\n");
            for (i, x) in v.iter().enumerate() {
                out.push_str(&pad);
                out.push_str("  ");
                pretty(x, ind + 1, out);
                out.push_str(if i + 1 < v.len() { ",\n" } else { "\n" });
            }
            write!(out, "{}]", pad).unwrap();
        }
        Json::Obj(v) => {
            out.push_str("{\n");
            for (i, (k, x)) in v.iter().enumerate() {
                write!(out, "{}  {:?}: ", pad, k).unwrap();
                pretty(x, ind + 1, out);
                out.push_str(if i + 1 < v.len() { ",\n" } else { "\n" });
            }
            write!(out, "{}}}", pad).unwrap();
        }
    }
}

fn compact(j: &Json) -> String {
    match j {
        Json::Null => "null".into(),
        Json::Bool(b) => b.to_string(),
        Json::Num(n) => num(*n),
        Json::Str(s) => format!("{:?}", s),
        Json::Arr(v) => format!("[{}]", v.iter().map(compact).collect::<Vec<_>>().join(",")),
        Json::Obj(v) => format!("{{{}}}", v.iter().map(|(k, x)| format!("{:?}:{}", k, compact(x))).collect::<Vec<_>>().join(",")),
    }
}

fn get<'a>(j: &'a Json, path: &str) -> Option<&'a Json> {
    path.split('.').filter(|s| !s.is_empty()).try_fold(j, |cur, seg| match cur {
        Json::Obj(v) => v.iter().find(|(k, _)| k == seg).map(|(_, x)| x),
        Json::Arr(v) => seg.parse::<usize>().ok().and_then(|i| v.get(i)),
        _ => None,
    })
}

fn main() {
    let src = r#" {"name": "rustgo", "version": 1.5e1, "tags": ["compiler", "go", "\u00e9t\u00e9"],
        "nested": {"empty_obj": {}, "empty_arr": [], "deep": [[1, 2], [3, [4, null]]]},
        "flags": {"fast": true, "safe": false}, "escape": "line\nbreak \"q\" \\ /", "neg": -0.25 } "#;
    let j = parse(src).unwrap();
    let mut out = String::new();
    pretty(&j, 0, &mut out);
    println!("{}", out);
    println!("{}", compact(&j));
    for path in ["name", "tags.2", "nested.deep.1.1.0", "nested.deep.1.1.1", "flags.safe", "nope", "tags.9", "version"] {
        println!("{} -> {:?}", path, get(&j, path).map(compact));
    }
    let again = parse(&compact(&j)).unwrap();
    println!("roundtrip equal: {}", again == j);
    for bad in ["[1, 2", "{\"a\" 1}", "tru", "[1,]", "\"abc", "{} x", "-", "[\"\\q\"]"] {
        println!("{:?} -> {:?}", bad, parse(bad));
    }
}
