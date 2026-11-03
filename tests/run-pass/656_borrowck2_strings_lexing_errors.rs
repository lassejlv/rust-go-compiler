// Valid programs that a naive borrow checker / front end might reject.
// Each module is an independent program; main runs them in order.

mod p43_let_else_and_if_let_chains {
    #[derive(Debug)]
    enum Tok<'a> { Num(i64), Word(&'a str), Sym(char) }
    fn lex(s: &str) -> Vec<Tok<'_>> {
        let mut out = Vec::new();
        for part in s.split_whitespace() {
            if let Ok(n) = part.parse::<i64>() { out.push(Tok::Num(n)); continue; }
            let mut cs = part.chars();
            let Some(c) = cs.next() else { continue };
            if cs.next().is_none() && !c.is_alphanumeric() { out.push(Tok::Sym(c)); } else { out.push(Tok::Word(part)); }
        }
        out
    }
    fn eval(toks: &[Tok]) -> i64 {
        let [Tok::Num(a), Tok::Sym(op), Tok::Num(b), ..] = toks else { return -1 };
        match op { '+' => a + b, '*' => a * b, _ => 0 }
    }
    pub fn main() {
        let src = String::from("3 * 14 hello ;");
        let toks = lex(&src);
        let words: Vec<&str> = toks.iter().filter_map(|t| if let Tok::Word(w) = t { Some(*w) } else { None }).collect();
        println!("{:?} {} {:?}", toks, eval(&toks), words);
    }
}

mod p57_string_processing_borrows {
    fn split_kv(line: &str) -> Option<(&str, &str)> { let (k, v) = line.split_once('=')?; Some((k.trim(), v.trim())) }
    fn longest_line(text: &str) -> &str { text.lines().max_by_key(|l| l.len()).unwrap_or("") }
    fn capitalize(words: &mut Vec<String>) { for w in words.iter_mut() { if let Some(f) = w.get_mut(0..1) { f.make_ascii_uppercase(); } } }
    pub fn main() {
        let text = String::from("a = 1\nlonger_key = value\nbad line");
        let pairs: Vec<(&str, &str)> = text.lines().filter_map(split_kv).collect();
        let ll = longest_line(&text);
        let mut words: Vec<String> = text.split_whitespace().map(String::from).collect();
        capitalize(&mut words);
        let mut s = String::new();
        for (i, (k, v)) in pairs.iter().enumerate() { if i > 0 { s.push(';'); } s += k; s += ":"; s.push_str(v); }
        let bytes = s.as_bytes();
        let colon = bytes.iter().position(|&b| b == b':').unwrap();
        let head = &s[..colon];
        println!("{:?} {} {:?} {} {}", pairs, ll, words, s, head);
    }
}

mod p39_question_mark_from {
    use std::fmt;
    #[derive(Debug)]
    enum AppErr { Parse(std::num::ParseIntError), Neg(i64) }
    impl From<std::num::ParseIntError> for AppErr { fn from(e: std::num::ParseIntError) -> Self { AppErr::Parse(e) } }
    impl fmt::Display for AppErr {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            match self { AppErr::Parse(e) => write!(f, "parse: {}", e), AppErr::Neg(n) => write!(f, "neg {}", n) }
        }
    }
    fn parse_pos(s: &str) -> Result<i64, AppErr> {
        let n: i64 = s.trim().parse()?;
        if n < 0 { return Err(AppErr::Neg(n)); }
        Ok(n)
    }
    fn first_char_upper(s: &str) -> Option<char> { let c = s.chars().next()?; Some(c.to_ascii_uppercase()) }
    fn sum_strs(xs: &[&str]) -> Result<i64, AppErr> { xs.iter().map(|s| parse_pos(s)).sum() }
    pub fn main() {
        for s in ["12", " 7 ", "-3", "x"] {
            match parse_pos(s) { Ok(n) => println!("ok {}", n), Err(e) => println!("err {}", e) }
        }
        let Some(c) = first_char_upper("abc") else { panic!() };
        let total = sum_strs(&["1", "2"]).unwrap_or(-1);
        let bad = sum_strs(&["1", "-2"]).is_err();
        println!("{} {:?} {} {}", c, first_char_upper(""), total, bad);
    }
}

fn main() {
    p43_let_else_and_if_let_chains::main();
    p57_string_processing_borrows::main();
    p39_question_mark_from::main();
}
