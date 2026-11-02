use std::collections::BTreeMap;
use std::fmt;

#[derive(Debug, Clone, PartialEq)]
enum Json {
    Null,
    Bool(bool),
    Num(f64),
    Str(String),
    Arr(Vec<Json>),
    Obj(BTreeMap<String, Json>),
}

impl fmt::Display for Json {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Json::Null => write!(f, "null"),
            Json::Bool(b) => write!(f, "{}", b),
            Json::Num(n) => write!(f, "{}", n),
            Json::Str(s) => write!(f, "\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\"")),
            Json::Arr(items) => {
                write!(f, "[")?;
                for (i, it) in items.iter().enumerate() {
                    if i > 0 {
                        write!(f, ",")?;
                    }
                    write!(f, "{}", it)?;
                }
                write!(f, "]")
            }
            Json::Obj(m) => {
                write!(f, "{{")?;
                for (i, (k, v)) in m.iter().enumerate() {
                    if i > 0 {
                        write!(f, ",")?;
                    }
                    write!(f, "\"{}\":{}", k, v)?;
                }
                write!(f, "}}")
            }
        }
    }
}

struct P<'a> {
    s: &'a [u8],
    i: usize,
}

impl<'a> P<'a> {
    fn ws(&mut self) {
        while self.i < self.s.len() && (self.s[self.i] as char).is_whitespace() {
            self.i += 1;
        }
    }
    fn expect(&mut self, c: u8) -> Result<(), String> {
        self.ws();
        if self.s.get(self.i) == Some(&c) {
            self.i += 1;
            Ok(())
        } else {
            Err(format!("expected '{}' at {}", c as char, self.i))
        }
    }
    fn value(&mut self) -> Result<Json, String> {
        self.ws();
        match self.s.get(self.i) {
            None => Err("unexpected end".to_string()),
            Some(b'n') => self.lit("null", Json::Null),
            Some(b't') => self.lit("true", Json::Bool(true)),
            Some(b'f') => self.lit("false", Json::Bool(false)),
            Some(b'"') => Ok(Json::Str(self.string()?)),
            Some(b'[') => {
                self.i += 1;
                let mut v = Vec::new();
                self.ws();
                if self.s.get(self.i) == Some(&b']') {
                    self.i += 1;
                    return Ok(Json::Arr(v));
                }
                loop {
                    v.push(self.value()?);
                    self.ws();
                    match self.s.get(self.i) {
                        Some(b',') => self.i += 1,
                        Some(b']') => {
                            self.i += 1;
                            return Ok(Json::Arr(v));
                        }
                        _ => return Err(format!("bad array at {}", self.i)),
                    }
                }
            }
            Some(b'{') => {
                self.i += 1;
                let mut m = BTreeMap::new();
                self.ws();
                if self.s.get(self.i) == Some(&b'}') {
                    self.i += 1;
                    return Ok(Json::Obj(m));
                }
                loop {
                    self.ws();
                    let k = self.string()?;
                    self.expect(b':')?;
                    let v = self.value()?;
                    m.insert(k, v);
                    self.ws();
                    match self.s.get(self.i) {
                        Some(b',') => self.i += 1,
                        Some(b'}') => {
                            self.i += 1;
                            return Ok(Json::Obj(m));
                        }
                        _ => return Err(format!("bad object at {}", self.i)),
                    }
                }
            }
            Some(c) if *c == b'-' || c.is_ascii_digit() => {
                let start = self.i;
                self.i += 1;
                while self.i < self.s.len() && (self.s[self.i].is_ascii_digit() || b".eE+-".contains(&self.s[self.i])) {
                    self.i += 1;
                }
                let txt = std::str::from_utf8(&self.s[start..self.i]).unwrap();
                txt.parse::<f64>().map(Json::Num).map_err(|e| format!("bad number {:?}: {}", txt, e))
            }
            Some(c) => Err(format!("unexpected '{}' at {}", *c as char, self.i)),
        }
    }
    fn lit(&mut self, word: &str, v: Json) -> Result<Json, String> {
        if self.s[self.i..].starts_with(word.as_bytes()) {
            self.i += word.len();
            Ok(v)
        } else {
            Err(format!("bad literal at {}", self.i))
        }
    }
    fn string(&mut self) -> Result<String, String> {
        if self.s.get(self.i) != Some(&b'"') {
            return Err(format!("expected string at {}", self.i));
        }
        self.i += 1;
        let mut out = String::new();
        while let Some(&c) = self.s.get(self.i) {
            self.i += 1;
            match c {
                b'"' => return Ok(out),
                b'\\' => {
                    let e = self.s[self.i];
                    self.i += 1;
                    out.push(match e {
                        b'n' => '\n',
                        b't' => '\t',
                        other => other as char,
                    });
                }
                _ => out.push(c as char),
            }
        }
        Err("unterminated string".to_string())
    }
}

fn parse(s: &str) -> Result<Json, String> {
    let mut p = P { s: s.as_bytes(), i: 0 };
    let v = p.value()?;
    p.ws();
    if p.i != s.len() {
        return Err(format!("trailing data at {}", p.i));
    }
    Ok(v)
}

fn depth(j: &Json) -> usize {
    match j {
        Json::Arr(v) => 1 + v.iter().map(depth).max().unwrap_or(0),
        Json::Obj(m) => 1 + m.values().map(depth).max().unwrap_or(0),
        _ => 0,
    }
}

fn sum_numbers(j: &Json) -> f64 {
    match j {
        Json::Num(n) => *n,
        Json::Arr(v) => v.iter().map(sum_numbers).sum(),
        Json::Obj(m) => m.values().map(sum_numbers).sum(),
        _ => 0.0,
    }
}

fn main() {
    let src = r#" { "name": "rustgo", "version": 1.5, "tags": ["compiler", "go", "rust"],
        "deps": { "none": null, "ok": true, "nested": [1, [2, [3, -4.25e1]]] },
        "quote": "say \"hi\"\n" } "#;
    match parse(src) {
        Ok(j) => {
            println!("{}", j);
            println!("depth = {}", depth(&j));
            println!("sum = {}", sum_numbers(&j));
            if let Json::Obj(m) = &j {
                println!("keys = {:?}", m.keys().collect::<Vec<_>>());
                println!("tags = {}", m["tags"]);
                println!("debug name = {:?}", m.get("name"));
            }
            let again = parse(&j.to_string()).unwrap();
            println!("roundtrip equal = {}", again == j);
        }
        Err(e) => println!("error: {}", e),
    }
    for bad in ["[1, 2", "{\"a\" 1}", "tru", "[1,]", "\"open", "12 34", "-"] {
        println!("{:<10} -> {:?}", bad, parse(bad).err());
    }
    println!("{}", parse("[]").unwrap());
    println!("{}", parse("{}").unwrap());
    println!("{}", parse("  -0.5 ").unwrap());
}
