fn first_word(s: &str) -> &str {
    let bytes = s.as_bytes();
    for (i, &b) in bytes.iter().enumerate() {
        if b == b' ' {
            return &s[..i];
        }
    }
    &s[..]
}

fn middle(s: &str) -> &str {
    let n = s.len();
    &s[n / 4..n - n / 4]
}

fn main() {
    let s = String::from("hello world program");
    println!("first word = {}", first_word(&s));
    println!("first word of literal = {}", first_word("single"));
    println!("&s[0..5] = {}", &s[0..5]);
    println!("&s[6..] = {}", &s[6..]);
    println!("&s[..=4] = {}", &s[..=4]);
    println!("middle = {}", middle("abcdefgh"));
    let st: &str = s.as_str();
    println!("as_str len = {}", st.len());
    let owned: String = st[6..11].to_string();
    println!("owned slice = {}", owned);
    let utf = "Grüße";
    println!("utf slice = {}", &utf[0..4]);
    println!("get(0..4) = {:?}", utf.get(0..4));
    println!("get(0..3) = {:?}", utf.get(0..3));
    println!("cmp: {} {} {}", "apple" < "banana", "Zebra" < "apple", "abc" < "abd");
    println!("cmp len: {}", "ab" < "abc");
    println!("String vs &str: {}", s == "hello world program");
    let mut names = vec!["delta", "Alpha", "charlie", "Echo", "bravo"];
    names.sort();
    println!("sorted = {:?}", names);
    names.sort_by_key(|n| n.to_lowercase());
    println!("case-insensitive = {:?}", names);
    let words: Vec<&str> = s.split(' ').collect();
    let longest = words.iter().fold("", |acc, w| if w.len() > acc.len() { w } else { acc });
    println!("longest = {}", longest);
    let chars: Vec<char> = "abc".chars().collect();
    let back: String = chars.iter().collect();
    println!("roundtrip = {}", back);
    println!("char_slice = {:?}", &chars[1..]);
    let substr_idx = s.find("world").unwrap();
    println!("after world = {}", &s[substr_idx + 5..].trim());
}
