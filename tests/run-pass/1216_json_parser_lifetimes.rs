use std::collections::BTreeMap;
use std::fmt;
use std::iter::Peekable;
use std::str::CharIndices;

#[derive(Debug, Clone, PartialEq)]
enum Json<'a> {
    Null,
    Bool(bool),
    Num(f64),
    Str(std::borrow::Cow<'a, str>),
    Arr(Vec<Json<'a>>),
    Obj(BTreeMap<&'a str, Json<'a>>),
}

#[derive(Debug)]
struct ParseError {
    pos: usize,
    msg: String,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "at {}: {}", self.pos, self.msg)
    }
}

struct Parser<'a> {
    src: &'a str,
    it: Peekable<CharIndices<'a>>,
}

type PResult<'a, T> = Result<T, ParseError>;

impl<'a> Parser<'a> {
    fn new(src: &'a str) -> Self {
        Parser { src, it: src.char_indices().peekable() }
    }
    fn pos(&mut self) -> usize {
        self.it.peek().map(|&(i, _)| i).unwrap_or(self.src.len())
    }
    fn err<T>(&mut self, msg: impl Into<String>) -> PResult<'a, T> {
        Err(ParseError { pos: self.pos(), msg: msg.into() })
    }
    fn ws(&mut self) {
        while self.it.next_if(|&(_, c)| c.is_whitespace()).is_some() {}
    }
    fn expect(&mut self, ch: char) -> PResult<'a, ()> {
        self.ws();
        match self.it.next() {
            Some((_, c)) if c == ch => Ok(()),
            Some((i, c)) => Err(ParseError { pos: i, msg: format!("expected {:?}, got {:?}", ch, c) }),
            None => self.err(format!("expected {:?}, got EOF", ch)),
        }
    }
    fn value(&mut self) -> PResult<'a, Json<'a>> {
        self.ws();
        match self.it.peek().copied() {
            None => self.err("unexpected EOF"),
            Some((_, '{')) => self.object(),
            Some((_, '[')) => self.array(),
            Some((_, '"')) => Ok(Json::Str(self.string()?)),
            Some((_, 't')) => self.keyword("true", Json::Bool(true)),
            Some((_, 'f')) => self.keyword("false", Json::Bool(false)),
            Some((_, 'n')) => self.keyword("null", Json::Null),
            Some((_, c)) if c == '-' || c.is_ascii_digit() => self.number(),
            Some((_, c)) => self.err(format!("unexpected char {:?}", c)),
        }
    }
    fn keyword(&mut self, kw: &str, v: Json<'a>) -> PResult<'a, Json<'a>> {
        let start = self.pos();
        if self.src[start..].starts_with(kw) {
            for _ in 0..kw.len() {
                self.it.next();
            }
            Ok(v)
        } else {
            self.err(format!("bad keyword, wanted {}", kw))
        }
    }
    fn number(&mut self) -> PResult<'a, Json<'a>> {
        let start = self.pos();
        let mut end = start;
        while let Some((i, c)) = self.it.next_if(|&(_, c)| c == '-' || c == '+' || c == '.' || c == 'e' || c == 'E' || c.is_ascii_digit()) {
            end = i + c.len_utf8();
        }
        match self.src[start..end].parse::<f64>() {
            Ok(n) => Ok(Json::Num(n)),
            Err(e) => Err(ParseError { pos: start, msg: e.to_string() }),
        }
    }
    fn raw_string(&mut self) -> PResult<'a, &'a str> {
        self.expect('"')?;
        let start = self.pos();
        loop {
            match self.it.next() {
                Some((i, '"')) => return Ok(&self.src[start..i]),
                Some((_, '\\')) => {
                    self.it.next();
                }
                Some(_) => {}
                None => return self.err("unterminated string"),
            }
        }
    }
    fn string(&mut self) -> PResult<'a, std::borrow::Cow<'a, str>> {
        let raw = self.raw_string()?;
        if !raw.contains('\\') {
            return Ok(std::borrow::Cow::Borrowed(raw));
        }
        let mut out = String::with_capacity(raw.len());
        let mut chars = raw.chars();
        while let Some(c) = chars.next() {
            if c != '\\' {
                out.push(c);
                continue;
            }
            match chars.next() {
                Some('n') => out.push('\n'),
                Some('t') => out.push('\t'),
                Some('u') => {
                    let hex: String = chars.by_ref().take(4).collect();
                    let cp = u32::from_str_radix(&hex, 16).map_err(|e| ParseError { pos: 0, msg: e.to_string() })?;
                    out.push(char::from_u32(cp).unwrap_or('?'));
                }
                Some(o) => out.push(o),
                None => {}
            }
        }
        Ok(std::borrow::Cow::Owned(out))
    }
    fn array(&mut self) -> PResult<'a, Json<'a>> {
        self.expect('[')?;
        let mut items = vec![];
        self.ws();
        if self.it.next_if(|&(_, c)| c == ']').is_some() {
            return Ok(Json::Arr(items));
        }
        loop {
            items.push(self.value()?);
            self.ws();
            match self.it.next() {
                Some((_, ',')) => continue,
                Some((_, ']')) => break,
                _ => return self.err("expected , or ]"),
            }
        }
        Ok(Json::Arr(items))
    }
    fn object(&mut self) -> PResult<'a, Json<'a>> {
        self.expect('{')?;
        let mut m = BTreeMap::new();
        self.ws();
        if self.it.next_if(|&(_, c)| c == '}').is_some() {
            return Ok(Json::Obj(m));
        }
        loop {
            self.ws();
            let k = self.raw_string()?;
            self.expect(':')?;
            let v = self.value()?;
            m.insert(k, v);
            self.ws();
            match self.it.next() {
                Some((_, ',')) => {}
                Some((_, '}')) => break,
                _ => return self.err("expected , or }"),
            }
        }
        Ok(Json::Obj(m))
    }
}

impl fmt::Display for Json<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Json::Null => f.write_str("null"),
            Json::Bool(b) => write!(f, "{}", b),
            Json::Num(n) => write!(f, "{}", n),
            Json::Str(s) => write!(f, "{:?}", s),
            Json::Arr(a) => {
                f.write_str("[")?;
                for (i, x) in a.iter().enumerate() {
                    if i > 0 {
                        f.write_str(",")?;
                    }
                    write!(f, "{}", x)?;
                }
                f.write_str("]")
            }
            Json::Obj(m) => {
                f.write_str("{")?;
                let mut first = true;
                for (k, v) in m {
                    if !std::mem::replace(&mut first, false) {
                        f.write_str(",")?;
                    }
                    write!(f, "{:?}:{}", k, v)?;
                }
                f.write_str("}")
            }
        }
    }
}

impl<'a> Json<'a> {
    fn get(&self, path: &[&str]) -> Option<&Json<'a>> {
        match path.split_first() {
            None => Some(self),
            Some((head, rest)) => match self {
                Json::Obj(m) => m.iter().find(|(k, _)| **k == *head)?.1.get(rest),
                Json::Arr(a) => a.get(head.parse::<usize>().ok()?)?.get(rest),
                _ => None,
            },
        }
    }
    fn depth(&self) -> usize {
        match self {
            Json::Arr(a) => 1 + a.iter().map(Json::depth).max().unwrap_or(0),
            Json::Obj(m) => 1 + m.values().map(|v| v.depth()).max().unwrap_or(0),
            _ => 0,
        }
    }
}

fn main() {
    let src = r#"
    {
        "name": "rustgo",
        "version": 1.5,
        "tags": ["compiler", "go", "c"],
        "nested": {"a": [1, 2, {"deep": true}], "b": null, "esc": "line\nnext é \"q\""},
        "neg": -2.5e3,
        "empty": [],
        "eobj": {}
    }"#;
    let mut p = Parser::new(src);
    match p.value() {
        Ok(j) => {
            println!("{}", j);
            println!("depth={}", j.depth());
            println!("{:?}", j.get(&["nested", "a", "2", "deep"]));
            println!("{:?}", j.get(&["tags", "1"]).map(|t| t.to_string()));
            println!("{:?}", j.get(&["nested", "esc"]));
            println!("{:?}", j.get(&["missing"]));
            if let Some(Json::Str(s)) = j.get(&["name"]) {
                println!("borrowed: {}", matches!(s, std::borrow::Cow::Borrowed(_)));
            }
            let j2 = j.clone();
            println!("{}", j == j2);
        }
        Err(e) => println!("error {}", e),
    }
    for bad in ["[1, 2", "{\"a\" 1}", "tru", "[1,]", "\"abc", "-", "{\"k\": [nul]}"] {
        match Parser::new(bad).value() {
            Ok(v) => println!("ok?? {}", v),
            Err(e) => println!("{:<14} -> {}", bad, e),
        }
    }
}
