// let-else, matches! with guards, if let ... else chains, while let with stack
#[derive(Debug, Clone)]
enum Token {
    Num(f64),
    Ident(String),
    Op(char),
    LParen,
    RParen,
}

fn parse_kv(line: &str) -> Result<(String, i64), String> {
    let Some((k, v)) = line.split_once('=') else {
        return Err(format!("no '=' in {:?}", line));
    };
    let k = k.trim();
    let Ok(v) = v.trim().parse::<i64>() else {
        return Err(format!("bad value for {}", k));
    };
    if k.is_empty() {
        return Err("empty key".into());
    }
    Ok((k.to_string(), v))
}

fn first_even(v: &[i32]) -> i32 {
    let [first, ..] = v else { return -1 };
    let Some(&e) = v.iter().find(|x| *x % 2 == 0) else {
        return *first * 1000;
    };
    e
}

fn tokenize(s: &str) -> Vec<Token> {
    let mut out = vec![];
    let cs: Vec<char> = s.chars().collect();
    let mut i = 0;
    while i < cs.len() {
        let c = cs[i];
        if matches!(c, ' ' | '\t') {
            i += 1;
            continue;
        }
        if c.is_ascii_digit() || (c == '.' && matches!(cs.get(i + 1), Some(d) if d.is_ascii_digit())) {
            let st = i;
            while i < cs.len() && (cs[i].is_ascii_digit() || cs[i] == '.') {
                i += 1;
            }
            out.push(Token::Num(cs[st..i].iter().collect::<String>().parse().unwrap()));
            continue;
        }
        if c.is_alphabetic() {
            let st = i;
            while i < cs.len() && cs[i].is_alphanumeric() {
                i += 1;
            }
            out.push(Token::Ident(cs[st..i].iter().collect()));
            continue;
        }
        out.push(match c {
            '(' => Token::LParen,
            ')' => Token::RParen,
            _ => Token::Op(c),
        });
        i += 1;
    }
    out
}

fn main() {
    for l in ["a = 5", "b=  -12", "novalue", "c = x", " = 3"] {
        println!("{:?}", parse_kv(l));
    }
    println!("{} {} {}", first_even(&[]), first_even(&[3, 5]), first_even(&[3, 8, 4]));
    let toks = tokenize("x1 + (3.5 * .25) - foo/2");
    println!("{:?}", toks);
    let nums = toks.iter().filter(|t| matches!(t, Token::Num(n) if *n < 1.0)).count();
    let ops: String = toks.iter().filter_map(|t| if let Token::Op(c) = t { Some(*c) } else { None }).collect();
    println!("{} {:?}", nums, ops);
    let parens_ok = {
        let mut depth = 0i32;
        toks.iter().all(|t| {
            match t {
                Token::LParen => depth += 1,
                Token::RParen => depth -= 1,
                _ => {}
            }
            depth >= 0
        }) && depth == 0
    };
    println!("balanced {}", parens_ok);
    println!("{}", matches!(toks.first(), Some(Token::Ident(s)) if s.starts_with('x')));
    println!("{}", matches!(toks.last(), Some(Token::Num(n)) if *n == 2.0));
    let chars = ['a', 'Z', '5', '_', ' '];
    let classes: Vec<&str> = chars
        .iter()
        .map(|c| match c {
            c if matches!(c, 'a'..='z') => "lower",
            'A'..='Z' => "upper",
            '0'..='9' => "digit",
            _ if c.is_whitespace() => "space",
            _ => "other",
        })
        .collect();
    println!("{:?}", classes);
    let mut stack = vec![1, 2, 3];
    while let Some(top) = stack.pop() {
        if top == 2 {
            stack.push(10);
            stack.push(20);
        }
        print!("{} ", top);
    }
    println!();
    let opts = [Some(3), None, Some(-1)];
    for o in opts {
        let msg = if let Some(x @ 1..) = o {
            format!("positive {}", x)
        } else if let Some(x) = o {
            format!("non-positive {}", x)
        } else {
            "none".to_string()
        };
        println!("{}", msg);
    }
    let data: Vec<Result<i32, String>> = vec![Ok(1), Err("x".into()), Ok(5)];
    let mut iter = data.iter();
    while let Some(Ok(v)) = iter.next() {
        println!("ok {}", v);
    }
    println!("rest {:?}", iter.collect::<Vec<_>>());
    let v: Option<Vec<i32>> = Some(vec![]);
    let Some(ref inner) = v else { unreachable!() };
    println!("{} {}", inner.is_empty(), matches!(v, Some(ref x) if x.is_empty()));
    let t = (1, "two", 3.0);
    let (a, b, c) = t else { panic!() };
    println!("{} {} {}", a, b, c);
}
