#[derive(Debug, Clone, Copy, PartialEq)]
enum Kind { Word, Number, Punct, Space }

#[derive(Debug, Clone, Copy)]
struct Token<'a> { text: &'a str, kind: Kind, offset: usize }

struct Lexer<'a> { src: &'a str, pos: usize }

impl<'a> Lexer<'a> {
    fn new(src: &'a str) -> Self { Lexer { src, pos: 0 } }
}

impl<'a> Iterator for Lexer<'a> {
    type Item = Token<'a>;
    fn next(&mut self) -> Option<Token<'a>> {
        let rest = &self.src[self.pos..];
        let first = rest.chars().next()?;
        let kind = if first.is_alphabetic() { Kind::Word } else if first.is_ascii_digit() { Kind::Number } else if first.is_whitespace() { Kind::Space } else { Kind::Punct };
        let len = match kind {
            Kind::Punct => first.len_utf8(),
            _ => rest.char_indices().find(|&(_, c)| {
                let k = if c.is_alphabetic() { Kind::Word } else if c.is_ascii_digit() { Kind::Number } else if c.is_whitespace() { Kind::Space } else { Kind::Punct };
                k != kind
            }).map(|(i, _)| i).unwrap_or(rest.len()),
        };
        let tok = Token { text: &rest[..len], kind, offset: self.pos };
        self.pos += len;
        Some(tok)
    }
}

struct Doc<'a> { title: &'a str, words: Vec<&'a str> }

fn longest<'a, 'b: 'a>(a: &'a str, b: &'b str) -> &'a str { if b.len() > a.len() { b } else { a } }

fn split_pair<'a>(s: &'a str, sep: char) -> (&'a str, Option<&'a str>) {
    match s.find(sep) { Some(i) => (&s[..i], Some(&s[i + sep.len_utf8()..])), None => (s, None) }
}

struct Highest<'s, T> { items: &'s [T] }
impl<'s, T: PartialOrd> Highest<'s, T> {
    fn get(&self) -> Option<&'s T> {
        let mut best: Option<&'s T> = None;
        for it in self.items { if best.map_or(true, |b| it > b) { best = Some(it); } }
        best
    }
    fn top_n(&self, n: usize) -> Vec<&'s T> {
        let mut v: Vec<&'s T> = self.items.iter().collect();
        v.sort_by(|a, b| b.partial_cmp(a).unwrap());
        v.truncate(n);
        v
    }
}

struct Config<'a> { entries: Vec<(&'a str, &'a str)> }
impl<'a> Config<'a> {
    fn parse(src: &'a str) -> Config<'a> {
        let entries = src.lines().map(str::trim).filter(|l| !l.is_empty() && !l.starts_with('#'))
            .filter_map(|l| l.split_once('=')).map(|(k, v)| (k.trim(), v.trim())).collect();
        Config { entries }
    }
    fn get(&self, k: &str) -> Option<&'a str> { self.entries.iter().rev().find(|(kk, _)| *kk == k).map(|(_, v)| *v) }
    fn with_prefix<'p>(&'p self, p: &'p str) -> impl Iterator<Item = &'a str> + 'p { self.entries.iter().filter(move |(k, _)| k.starts_with(p)).map(|(_, v)| *v) }
}

fn first_word_mut<'a>(words: &'a mut Vec<String>) -> Option<&'a mut String> { words.iter_mut().find(|w| !w.is_empty()) }

fn main() {
    let text = "Hello, world! 42 apples & 7 pears—fresh. Ünïcode wörds 2024";
    let toks: Vec<Token> = Lexer::new(text).filter(|t| t.kind != Kind::Space).collect();
    for t in &toks { println!("{:>3} {:<7} {:?}", t.offset, format!("{:?}", t.kind), t.text); }
    let numbers: u32 = Lexer::new(text).filter(|t| t.kind == Kind::Number).map(|t| t.text.parse::<u32>().unwrap()).sum();
    println!("sum of numbers {}", numbers);
    let doc = Doc { title: &text[..5], words: toks.iter().filter(|t| t.kind == Kind::Word).map(|t| t.text).collect() };
    println!("title={} words={:?}", doc.title, doc.words);
    let l;
    {
        let a = String::from("short");
        l = longest(doc.words[0], doc.words[1]);
        println!("longest in scope {}", longest(&a, "tiny"));
    }
    println!("longest {}", l);
    println!("{:?} {:?} {:?}", split_pair("key=value=x", '='), split_pair("novalue", '='), split_pair("a→b", '→'));
    let nums = [3.5, 9.25, -1.0, 9.0, 4.75];
    let h = Highest { items: &nums };
    let best = h.get();
    let top = h.top_n(3);
    println!("best {:?} top {:?}", best, top);
    let strs = vec!["pear".to_string(), "zebra".into(), "apple".into()];
    let hs = Highest { items: &strs[..] };
    println!("best str {:?}", hs.get());
    let src = String::from("# comment\nname = demo\nport=8080\n\ndb.host = localhost\ndb.port = 5432\nname = override\n");
    let cfg = Config::parse(&src);
    println!("name={:?} port={:?} missing={:?}", cfg.get("name"), cfg.get("port"), cfg.get("nope"));
    let dbs: Vec<&str> = cfg.with_prefix("db.").collect();
    println!("db values {:?}", dbs);
    let port: u16 = cfg.get("port").and_then(|p| p.parse().ok()).unwrap_or(80);
    println!("port+1 {}", port + 1);
    let mut words = vec![String::new(), "abc".into(), "def".into()];
    if let Some(w) = first_word_mut(&mut words) { w.push_str("XYZ"); }
    println!("{:?}", words);
    let joined: String = toks.iter().rev().take(4).map(|t| t.text).collect::<Vec<_>>().join("|");
    println!("{}", joined);
    let mut it = Lexer::new("a1 b2");
    let first_two: Vec<&str> = it.by_ref().take(2).map(|t| t.text).collect();
    let rest: Vec<&str> = it.map(|t| t.text).collect();
    println!("{:?} {:?}", first_two, rest);
}
