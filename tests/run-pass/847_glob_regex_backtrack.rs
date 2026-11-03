use std::collections::HashMap;

fn glob(p: &[u8], s: &[u8]) -> bool {
    let (mut pi, mut si) = (0, 0);
    let (mut star, mut mark) = (None, 0);
    while si < s.len() {
        if pi < p.len() && (p[pi] == b'?' || p[pi] == s[si]) { pi += 1; si += 1; }
        else if pi < p.len() && p[pi] == b'*' { star = Some(pi); mark = si; pi += 1; }
        else if let Some(st) = star { pi = st + 1; mark += 1; si = mark; }
        else { return false; }
    }
    while pi < p.len() && p[pi] == b'*' { pi += 1; }
    pi == p.len()
}

#[derive(Debug, Clone, PartialEq)]
enum Re { Char(char), Any, Class(Vec<(char, char)>, bool), Star(Box<Re>), Plus(Box<Re>), Opt(Box<Re>), Group(Vec<Re>), Alt(Vec<Vec<Re>>), Start, End }

fn parse(p: &str) -> Vec<Re> {
    let cs: Vec<char> = p.chars().collect();
    let mut i = 0;
    let r = parse_alt(&cs, &mut i);
    if r.len() == 1 { r.into_iter().next().unwrap() } else { vec![Re::Alt(r)] }
}
fn parse_alt(cs: &[char], i: &mut usize) -> Vec<Vec<Re>> {
    let mut alts = vec![parse_seq(cs, i)];
    while *i < cs.len() && cs[*i] == '|' { *i += 1; alts.push(parse_seq(cs, i)); }
    alts
}
fn parse_seq(cs: &[char], i: &mut usize) -> Vec<Re> {
    let mut out = vec![];
    while *i < cs.len() && cs[*i] != '|' && cs[*i] != ')' {
        let atom = match cs[*i] {
            '.' => { *i += 1; Re::Any }
            '^' => { *i += 1; Re::Start }
            '$' => { *i += 1; Re::End }
            '(' => { *i += 1; let a = parse_alt(cs, i); *i += 1; if a.len() == 1 { Re::Group(a.into_iter().next().unwrap()) } else { Re::Alt(a) } }
            '[' => {
                *i += 1;
                let neg = cs[*i] == '^'; if neg { *i += 1; }
                let mut ranges = vec![];
                while cs[*i] != ']' { let a = cs[*i]; if cs.get(*i + 1) == Some(&'-') && cs.get(*i + 2) != Some(&']') { ranges.push((a, cs[*i + 2])); *i += 3; } else { ranges.push((a, a)); *i += 1; } }
                *i += 1;
                Re::Class(ranges, neg)
            }
            '\\' => { *i += 2; match cs[*i - 1] { 'd' => Re::Class(vec![('0', '9')], false), 'w' => Re::Class(vec![('a', 'z'), ('A', 'Z'), ('0', '9'), ('_', '_')], false), 's' => Re::Class(vec![(' ', ' '), ('\t', '\t')], false), c => Re::Char(c) } }
            c => { *i += 1; Re::Char(c) }
        };
        let atom = match cs.get(*i) { Some('*') => { *i += 1; Re::Star(Box::new(atom)) } Some('+') => { *i += 1; Re::Plus(Box::new(atom)) } Some('?') => { *i += 1; Re::Opt(Box::new(atom)) } _ => atom };
        out.push(atom);
    }
    out
}

fn m_one(r: &Re, s: &[char], i: usize) -> bool {
    match r {
        Re::Char(c) => s.get(i) == Some(c),
        Re::Any => i < s.len(),
        Re::Class(rs, neg) => s.get(i).map_or(false, |c| rs.iter().any(|&(a, b)| a <= *c && *c <= b) != *neg),
        _ => false,
    }
}

fn m(rs: &[Re], s: &[char], i: usize, k: &mut dyn FnMut(usize) -> bool) -> bool {
    let Some((first, rest)) = rs.split_first() else { return k(i) };
    match first {
        Re::Start => i == 0 && m(rest, s, i, k),
        Re::End => i == s.len() && m(rest, s, i, k),
        Re::Group(g) => m(g, s, i, &mut |j| m(rest, s, j, k)),
        Re::Alt(alts) => alts.iter().any(|a| m(a, s, i, &mut |j| m(rest, s, j, k))),
        Re::Opt(inner) => m(std::slice::from_ref(inner.as_ref()), s, i, &mut |j| m(rest, s, j, k)) || m(rest, s, i, k),
        Re::Star(inner) | Re::Plus(inner) => {
            let plus = matches!(first, Re::Plus(_));
            let mut positions = vec![i];
            let mut cur = i;
            while m_one(inner, s, cur) { cur += 1; positions.push(cur); }
            if !matches!(**inner, Re::Char(_) | Re::Any | Re::Class(..)) {
                // complex inner: fall back to single greedy expansion via recursion
                return star_complex(inner, plus, rest, s, i, k, 0);
            }
            positions.iter().rev().filter(|&&p| !plus || p > i).any(|&p| m(rest, s, p, k))
        }
        _ => m_one(first, s, i) && m(rest, s, i + 1, k),
    }
}

fn star_complex(inner: &Re, plus: bool, rest: &[Re], s: &[char], i: usize, k: &mut dyn FnMut(usize) -> bool, depth: usize) -> bool {
    if depth > s.len() + 1 { return false; }
    let mut try_more = |j: usize| j > i && star_complex(inner, false, rest, s, j, k, depth + 1);
    if m(std::slice::from_ref(inner), s, i, &mut try_more) { return true; }
    !plus && m(rest, s, i, k)
}

fn find(re: &[Re], text: &str) -> Option<(usize, String)> {
    let s: Vec<char> = text.chars().collect();
    for start in 0..=s.len() {
        let mut end = None;
        if m(re, &s, start, &mut |j| { end = Some(j); true }) { return Some((start, s[start..end.unwrap()].iter().collect())); }
    }
    None
}

fn main() {
    let globs = [("*.rs", "main.rs"), ("*.rs", "main.rc"), ("a*b*c", "aXXbYYc"), ("a*b*c", "abcabd"), ("?at", "cat"), ("?at", "at"), ("**x", "x"), ("*", ""), ("a?c*", "abcdef"), ("*a*a*a*a*b", "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa")];
    for (p, s) in globs { println!("glob {:<12} {:<12} {}", p, if s.len() > 12 { &s[..12] } else { s }, glob(p.as_bytes(), s.as_bytes())); }
    let tests = [
        ("ab*c", "xxabbbcyy"), ("^ab*c", "xxabc"), ("colou?r", "my favourite color"), ("\\d+-\\d+", "call 555-1234 now"),
        ("[a-c]+[^a-c]", "zzabcabcd!"), ("(ab)+", "xxababab"), ("cat|dog|bird", "hotdog stand"), ("^(a|b)*c$", "ababbac"),
        ("^(a|b)*c$", "ababxc"), ("\\w+@\\w+\\.com", "mail bob_1@example.com today"), ("h.llo", "say hallo"), ("x*", "aaa"), ("(a|ab)(c|bcd)(d*)", "abcd"),
    ];
    let mut cache: HashMap<&str, Vec<Re>> = HashMap::new();
    for (p, t) in tests {
        let re = cache.entry(p).or_insert_with(|| parse(p));
        println!("{:<18} on {:<28} => {:?}", p, format!("{:?}", t), find(re, t));
    }
    println!("{:?}", parse("a(b|c)*[^x-z]?$"));
}
