#[derive(Debug, PartialEq)]
enum Tok {
    Num(i64),
    Ident(String),
    Sym(char),
    Str(String),
}

fn lex(src: &str) -> Vec<Tok> {
    let mut out = Vec::new();
    let mut it = src.chars().peekable();
    while let Some(&c) = it.peek() {
        if c.is_whitespace() {
            it.next();
        } else if c.is_ascii_digit() {
            let mut n = 0i64;
            while let Some(&d) = it.peek() {
                if let Some(v) = d.to_digit(10) {
                    n = n * 10 + v as i64;
                    it.next();
                } else {
                    break;
                }
            }
            out.push(Tok::Num(n));
        } else if c.is_alphabetic() || c == '_' {
            let mut s = String::new();
            while let Some(&d) = it.peek() {
                if d.is_alphanumeric() || d == '_' {
                    s.push(d);
                    it.next();
                } else {
                    break;
                }
            }
            out.push(Tok::Ident(s));
        } else if c == '"' {
            it.next();
            let s: String = it.by_ref().take_while(|&ch| ch != '"').collect();
            out.push(Tok::Str(s));
        } else {
            out.push(Tok::Sym(c));
            it.next();
        }
    }
    out
}

fn group_runs(v: &[i32]) -> Vec<(i32, usize)> {
    let mut out = Vec::new();
    let mut it = v.iter().peekable();
    while let Some(&x) = it.next() {
        let mut n = 1;
        while it.peek() == Some(&&x) {
            it.next();
            n += 1;
        }
        out.push((x, n));
    }
    out
}

fn main() {
    let toks = lex("let x_1 = 42 + foo(7, \"hi there\");");
    for t in &toks {
        println!("{:?}", t);
    }
    println!("count = {}", toks.len());
    println!("runs = {:?}", group_runs(&[1, 1, 2, 3, 3, 3, 1, 4, 4]));
    let mut p = [10, 20, 30].iter().peekable();
    println!("peek {:?}", p.peek());
    println!("next {:?}", p.next());
    println!("next_if >15 {:?}", p.next_if(|&&x| x > 15));
    println!("next_if >100 {:?}", p.next_if(|&&x| x > 100));
    println!("next_if_eq 30 {:?}", p.next_if_eq(&&30));
    println!("peek end {:?}", p.peek());
    let mut it = "aaabbc".chars().peekable();
    let mut compressed = String::new();
    while let Some(c) = it.next() {
        let mut count = 1;
        while it.next_if_eq(&c).is_some() {
            count += 1;
        }
        compressed.push_str(&format!("{}{}", c, count));
    }
    println!("compressed = {}", compressed);
}
