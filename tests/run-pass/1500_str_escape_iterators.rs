// str::escape_debug/escape_default/escape_unicode return iterators of chars
// (which also implement Display), not Strings.
use std::str::{EscapeDebug, EscapeDefault, EscapeUnicode};

fn count<I: Iterator<Item = char>>(it: I) -> usize {
    it.count()
}

fn main() {
    let s = "tab\there \"q\" é\n\u{7f}";
    let d: EscapeDebug = s.escape_debug();
    let e: EscapeDefault = s.escape_default();
    let u: EscapeUnicode = "ab".escape_unicode();
    println!("{}", d);
    println!("{}", e);
    println!("{}", u);
    println!("{:?}", "a\tb".escape_default().collect::<String>());
    println!("{:?}", "x\ny".escape_debug().collect::<Vec<char>>());
    println!("{}", count("é".escape_unicode()));
    let mut it = "\n!".escape_default();
    println!("{:?} {:?}", it.next(), it.next());
    // Display prints what is left
    println!("{}", it);
    println!("{:?}", it.size_hint().1.map(|n| n >= 1));
    let c = it.clone();
    println!("{}", c.collect::<String>());
    println!("{}", "\u{301}a\u{301}".escape_debug());
    println!("[{:>12}]", "q\"".escape_default().to_string());
    let total: usize = ["a", "\u{0}", "\u{10ffff}"].iter().map(|s| s.escape_debug().count()).sum();
    println!("{}", total);
    let s: String = "line\r\n".escape_debug().filter(|c| *c != '\\').collect();
    println!("{}", s);
}
