use std::collections::BTreeMap;
use std::fmt;

#[derive(Debug, Clone, PartialEq)]
enum Val<'s> { Str(&'s str), Int(i64), Bool(bool), List(Vec<Val<'s>>) }

#[derive(Debug)]
struct ParseError<'s> { line: usize, text: &'s str, msg: String }
impl<'s> fmt::Display for ParseError<'s> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result { write!(f, "line {}: {} (in {:?})", self.line, self.msg, self.text) }
}

type Section<'s> = BTreeMap<&'s str, Val<'s>>;

#[derive(Debug, Default)]
struct Doc<'s> { sections: BTreeMap<&'s str, Section<'s>> }

fn parse_val(s: &str) -> Result<Val<'_>, String> {
    let s = s.trim();
    if let Some(inner) = s.strip_prefix('[').and_then(|r| r.strip_suffix(']')) {
        if inner.trim().is_empty() { return Ok(Val::List(vec![])); }
        return inner.split(',').map(parse_val).collect::<Result<Vec<_>, _>>().map(Val::List);
    }
    if let Some(q) = s.strip_prefix('"') { return q.strip_suffix('"').map(Val::Str).ok_or_else(|| "unterminated string".to_string()); }
    match s { "true" => return Ok(Val::Bool(true)), "false" => return Ok(Val::Bool(false)), _ => {} }
    s.parse::<i64>().map(Val::Int).map_err(|e| format!("bad value {:?}: {}", s, e))
}

fn parse(src: &str) -> Result<Doc<'_>, Vec<ParseError<'_>>> {
    let mut doc = Doc::default();
    let mut current = "root";
    let mut errors = vec![];
    for (i, raw) in src.lines().enumerate() {
        let line = raw.split('#').next().unwrap_or("").trim();
        if line.is_empty() { continue; }
        if let Some(name) = line.strip_prefix('[').and_then(|l| l.strip_suffix(']')) {
            current = name.trim();
            doc.sections.entry(current).or_default();
            continue;
        }
        match line.split_once('=') {
            Some((k, v)) => match parse_val(v) {
                Ok(val) => { doc.sections.entry(current).or_default().insert(k.trim(), val); }
                Err(msg) => errors.push(ParseError { line: i + 1, text: raw, msg }),
            },
            None => errors.push(ParseError { line: i + 1, text: raw, msg: "expected key = value".into() }),
        }
    }
    if errors.is_empty() { Ok(doc) } else { Err(errors) }
}

impl<'s> Doc<'s> {
    fn get(&self, path: &str) -> Option<&Val<'s>> {
        let (sec, key) = path.rsplit_once('.').unwrap_or(("root", path));
        self.sections.get(sec)?.get(key)
    }
    fn get_str(&self, path: &str) -> Option<&'s str> { match self.get(path)? { Val::Str(s) => Some(*s), _ => None } }
    fn get_int(&self, path: &str) -> Option<i64> { if let Some(Val::Int(n)) = self.get(path) { Some(*n) } else { None } }
    fn keys_with_prefix<'p>(&'p self, prefix: &'p str) -> impl Iterator<Item = (&'s str, &'s str)> + 'p {
        self.sections.iter().filter(move |(s, _)| s.starts_with(prefix)).flat_map(|(s, sec)| sec.keys().map(move |k| (*s, *k)))
    }
}

fn longest_value<'a, 's>(doc: &'a Doc<'s>) -> Option<(&'s str, &'a Val<'s>)> {
    doc.sections.values().flat_map(|s| s.iter()).max_by_key(|(_, v)| format!("{:?}", v).len()).map(|(k, v)| (*k, v))
}

fn main() {
    let src = String::from(r#"
name = "demo"   # comment
version = 3
[server]
host = "localhost"
port = 8080
tls = false
[server.limits]
conns = 100
paths = ["/a", "/b", 7, true]
[client]
retries = 5
empty = []
"#);
    let doc = parse(&src).expect("valid");
    for (s, sec) in &doc.sections { println!("[{}] {:?}", s, sec); }
    println!("{:?} {:?} {:?} {:?}", doc.get_str("name"), doc.get_int("server.port"), doc.get_str("server.port"), doc.get("server.limits.paths"));
    println!("{:?}", doc.keys_with_prefix("server").collect::<Vec<_>>());
    println!("{:?}", longest_value(&doc));
    let name: &str = { let d = parse(&src).unwrap(); d.get_str("name").unwrap() };
    println!("name outlives doc: {}", name);
    let bad = "a = 1\nb = \"open\nc\n[s]\nd = [1, x]\ne = 99999999999999999999\nf = true";
    match parse(bad) {
        Ok(_) => println!("unexpected ok"),
        Err(es) => for e in &es { println!("{}", e); },
    }
}
