use std::borrow::Cow;

// Only allocate when escaping is actually needed.
fn escape_html(s: &str) -> Cow<'_, str> {
    if !s.contains(|c| matches!(c, '<' | '>' | '&' | '"')) {
        return Cow::Borrowed(s);
    }
    let mut out = String::with_capacity(s.len() + 8);
    for c in s.chars() {
        match c {
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '&' => out.push_str("&amp;"),
            '"' => out.push_str("&quot;"),
            c => out.push(c),
        }
    }
    Cow::Owned(out)
}

fn normalize_ws(s: &str) -> Cow<'_, str> {
    let needs = s.contains("  ") || s.starts_with(' ') || s.ends_with(' ') || s.contains('\t');
    if needs {
        Cow::Owned(s.split_whitespace().collect::<Vec<_>>().join(" "))
    } else {
        Cow::Borrowed(s)
    }
}

fn kind(c: &Cow<'_, str>) -> &'static str {
    match c {
        Cow::Borrowed(_) => "borrowed",
        Cow::Owned(_) => "owned",
    }
}

// Cow<[T]>: clamp values, cloning only if something changes
fn clamp_all(xs: &[i32], lo: i32, hi: i32) -> Cow<'_, [i32]> {
    let mut cow = Cow::Borrowed(xs);
    for i in 0..xs.len() {
        let v = xs[i];
        if v < lo || v > hi {
            cow.to_mut()[i] = v.clamp(lo, hi);
        }
    }
    cow
}

struct Config<'a> {
    name: Cow<'a, str>,
    paths: Vec<Cow<'a, str>>,
}

impl<'a> Config<'a> {
    fn new(name: impl Into<Cow<'a, str>>) -> Self {
        Config { name: name.into(), paths: vec![] }
    }
    fn path(mut self, p: impl Into<Cow<'a, str>>) -> Self {
        self.paths.push(p.into());
        self
    }
    fn into_owned(self) -> Config<'static> {
        Config {
            name: Cow::Owned(self.name.into_owned()),
            paths: self.paths.into_iter().map(|p| Cow::Owned(p.into_owned())).collect(),
        }
    }
    fn describe(&self) -> String {
        let kinds: Vec<&str> = self.paths.iter().map(kind).collect();
        format!("{}[{}] {:?}", self.name, kind(&self.name), kinds)
    }
}

fn make_static() -> Config<'static> {
    let owned = String::from("dynamic");
    let c = Config::new(owned.as_str()).path("/usr").path(format!("/home/{}", "me"));
    c.into_owned()
}

fn plural<'a>(word: &'a str, n: usize) -> Cow<'a, str> {
    if n == 1 {
        word.into()
    } else if let Some(stem) = word.strip_suffix('y') {
        format!("{}ies", stem).into()
    } else {
        (word.to_owned() + "s").into()
    }
}

fn main() {
    for s in ["plain text", "a < b && c > d", "say \"hi\""] {
        let e = escape_html(s);
        println!("{:<10} {}", kind(&e), e);
    }
    for s in ["one two", "  lots   of\tspace ", "x"] {
        let n = normalize_ws(s);
        println!("{:<10} {:?}", kind(&n), n);
    }

    let data = [1, 5, 10, 15];
    let c1 = clamp_all(&data, 0, 20);
    let c2 = clamp_all(&data, 2, 12);
    println!("{:?} {}", c1, matches!(c1, Cow::Borrowed(_)));
    println!("{:?} {}", c2, matches!(c2, Cow::Owned(_)));
    let owned_vec: Vec<i32> = c2.into_owned();
    println!("{:?}", owned_vec);

    let cfg = Config::new("app").path("/etc").path(String::from("/var/lib"));
    println!("{}", cfg.describe());
    let st = make_static();
    println!("{} paths={:?}", st.describe(), st.paths);

    for (w, n) in [("cat", 1), ("cat", 2), ("city", 3), ("key", 1)] {
        let p = plural(w, n);
        println!("{} {} ({})", n, p, kind(&p));
    }

    let mut c: Cow<str> = Cow::Borrowed("base");
    c.to_mut().push_str("+more");
    println!("{} {} len={}", c, kind(&c), c.len());
    let joined: Cow<str> = Cow::Borrowed("a") + "b";
    println!("{} {}", joined, kind(&joined));
    let lossy = String::from_utf8_lossy(b"ok bytes");
    let lossy2 = String::from_utf8_lossy(b"bad \xff byte");
    println!("{} {} / {} {}", lossy, kind(&lossy), lossy2, kind(&lossy2));
    let eq = Cow::Borrowed("x") == Cow::<str>::Owned("x".to_string());
    println!("eq={}", eq);
}
