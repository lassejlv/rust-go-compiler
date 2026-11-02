struct Parser<'a> {
    src: &'a str,
    pos: usize,
}

impl<'a> Parser<'a> {
    fn new(src: &'a str) -> Self {
        Parser { src, pos: 0 }
    }

    fn skip_ws(&mut self) {
        while self.pos < self.src.len() && self.src.as_bytes()[self.pos] == b' ' {
            self.pos += 1;
        }
    }

    fn next_word(&mut self) -> Option<&'a str> {
        self.skip_ws();
        if self.pos >= self.src.len() {
            return None;
        }
        let start = self.pos;
        while self.pos < self.src.len() && self.src.as_bytes()[self.pos] != b' ' {
            self.pos += 1;
        }
        Some(&self.src[start..self.pos])
    }

    fn rest(&self) -> &'a str {
        &self.src[self.pos..]
    }
}

struct Highlight<'t> {
    text: &'t str,
    start: usize,
    end: usize,
}

impl<'t> Highlight<'t> {
    fn excerpt(&self) -> &'t str {
        &self.text[self.start..self.end]
    }
}

fn find_all<'a>(haystack: &'a str, needle: &str) -> Vec<Highlight<'a>> {
    let mut out = Vec::new();
    let mut from = 0;
    while let Some(i) = haystack[from..].find(needle) {
        let s = from + i;
        out.push(Highlight { text: haystack, start: s, end: s + needle.len() });
        from = s + needle.len();
    }
    out
}

fn longest<'a>(x: &'a str, y: &'a str) -> &'a str {
    if x.len() >= y.len() { x } else { y }
}

fn first_of<'a, 'b>(x: &'a str, _y: &'b str) -> &'a str {
    x.split(',').next().unwrap_or("")
}

struct Pair<'a, T> {
    left: &'a T,
    right: &'a T,
}

impl<'a, T: PartialOrd> Pair<'a, T> {
    fn max(&self) -> &'a T {
        if self.left >= self.right { self.left } else { self.right }
    }
}

fn split_pair<'a>(s: &'a str, sep: char) -> (&'a str, &'a str) {
    match s.find(sep) {
        Some(i) => (&s[..i], &s[i + sep.len_utf8()..]),
        None => (s, ""),
    }
}

const STATIC_MSG: &'static str = "static lifetime";

fn main() {
    let text = String::from("  let   answer = 42 ;");
    let mut p = Parser::new(&text);
    let mut words = Vec::new();
    while let Some(w) = p.next_word() {
        words.push(w);
        if w == "=" {
            println!("rest after '=': {:?}", p.rest());
        }
    }
    println!("words = {:?}", words);
    let doc = "the cat sat on the mat with the hat";
    let hs = find_all(doc, "the");
    for h in &hs {
        println!("found '{}' at {}..{}", h.excerpt(), h.start, h.end);
    }
    let a = String::from("short");
    let result;
    {
        let b = String::from("much longer one");
        result = longest(&a, &b).to_string();
    }
    println!("longest = {}", result);
    println!("first_of = {}", first_of("x,y,z", "ignored"));
    let (x, y) = (10, 20);
    let pr = Pair { left: &x, right: &y };
    println!("pair max = {}", pr.max());
    let s1 = String::from("pear");
    let s2 = String::from("apple");
    let ps = Pair { left: &s1, right: &s2 };
    println!("pair max str = {}", ps.max());
    println!("split_pair = {:?}", split_pair("key=value", '='));
    println!("split_pair none = {:?}", split_pair("novalue", '='));
    println!("split_pair utf8 = {:?}", split_pair("a→b", '→'));
    println!("{}", STATIC_MSG);
}
