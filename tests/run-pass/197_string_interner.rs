use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
struct Sym(u32);

#[derive(Default)]
struct Interner {
    map: HashMap<String, Sym>,
    strings: Vec<String>,
}

impl Interner {
    fn intern(&mut self, s: &str) -> Sym {
        if let Some(&sym) = self.map.get(s) {
            return sym;
        }
        let sym = Sym(self.strings.len() as u32);
        self.strings.push(s.to_string());
        self.map.insert(s.to_string(), sym);
        sym
    }
    fn resolve(&self, s: Sym) -> &str {
        &self.strings[s.0 as usize]
    }
    fn len(&self) -> usize {
        self.strings.len()
    }
}

// A borrowing tokenizer: tokens are slices of the source with its lifetime.
struct Tokens<'src> {
    src: &'src str,
    pos: usize,
}

impl<'src> Iterator for Tokens<'src> {
    type Item = &'src str;
    fn next(&mut self) -> Option<&'src str> {
        let rest = &self.src[self.pos..];
        let start = rest.find(|c: char| !c.is_whitespace())?;
        let rest2 = &rest[start..];
        let len = if rest2.starts_with(|c: char| c.is_alphanumeric() || c == '_') {
            rest2.find(|c: char| !(c.is_alphanumeric() || c == '_')).unwrap_or(rest2.len())
        } else {
            rest2.chars().next().unwrap().len_utf8()
        };
        let tok = &rest2[..len];
        self.pos += start + len;
        Some(tok)
    }
}

struct Longest<'a> {
    best: Option<&'a str>,
}

impl<'a> Longest<'a> {
    fn offer(&mut self, s: &'a str) {
        if self.best.map_or(true, |b| s.len() > b.len()) {
            self.best = Some(s);
        }
    }
}

fn longer<'a>(a: &'a str, b: &'a str) -> &'a str {
    if b.len() > a.len() { b } else { a }
}

fn first_ident<'a>(toks: &[&'a str]) -> Option<&'a str> {
    toks.iter().copied().find(|t| t.chars().next().map_or(false, |c| c.is_alphabetic()))
}

struct SymbolTable<'i> {
    interner: &'i Interner,
    scopes: Vec<HashMap<Sym, i64>>,
}

impl<'i> SymbolTable<'i> {
    fn lookup(&self, s: Sym) -> Option<(usize, i64)> {
        self.scopes.iter().enumerate().rev().find_map(|(d, sc)| sc.get(&s).map(|v| (d, *v)))
    }
    fn describe(&self, s: Sym) -> String {
        match self.lookup(s) {
            Some((d, v)) => format!("{}@{}={}", self.interner.resolve(s), d, v),
            None => format!("{}=undefined", self.interner.resolve(s)),
        }
    }
}

fn main() {
    let src = "let x = foo(x, y_2) + x * bar; let y_2 = x; λ = x";
    let toks: Vec<&str> = Tokens { src, pos: 0 }.collect();
    println!("{} tokens: {:?}", toks.len(), toks);

    let mut interner = Interner::default();
    let syms: Vec<Sym> = toks.iter().map(|t| interner.intern(t)).collect();
    println!("unique {} of {}", interner.len(), syms.len());
    println!("{:?}", &syms[..8]);
    let x = interner.intern("x");
    println!("x = {:?}, occurrences {}", x, syms.iter().filter(|&&s| s == x).count());
    let mut counts: Vec<(Sym, usize)> = Vec::new();
    for s in &syms {
        match counts.iter_mut().find(|(k, _)| k == s) {
            Some((_, c)) => *c += 1,
            None => counts.push((*s, 1)),
        }
    }
    counts.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    let top: Vec<String> = counts.iter().take(4).map(|(s, c)| format!("{}:{}", interner.resolve(*s), c)).collect();
    println!("top {:?}", top);

    let mut l = Longest { best: None };
    for t in &toks {
        l.offer(t);
    }
    println!("longest token {:?}", l.best);
    println!("longer {}", longer(toks[3], toks[5]));
    println!("first ident {:?}", first_ident(&toks[2..]));
    println!("first ident none {:?}", first_ident(&["(", "=", ";"]));

    let y = interner.intern("y_2");
    let foo = interner.intern("foo");
    let undefined = interner.intern("never_seen");
    println!("new symbol {:?} total {}", undefined, interner.len());
    let mut table = SymbolTable { interner: &interner, scopes: vec![HashMap::new()] };
    table.scopes[0].insert(x, 1);
    table.scopes.push(HashMap::new());
    table.scopes[1].insert(y, 20);
    table.scopes[1].insert(x, 10);
    println!("{} {} {}", table.describe(x), table.describe(y), table.describe(foo));
    table.scopes.pop();
    println!("{} {}", table.describe(x), table.describe(y));
    let resolved: Vec<&str> = syms.iter().rev().take(3).map(|&s| interner.resolve(s)).collect();
    println!("{:?}", resolved);
}
