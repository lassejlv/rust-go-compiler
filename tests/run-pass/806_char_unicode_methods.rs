fn caesar(s: &str, k: u8) -> String {
    s.chars().map(|c| {
        if c.is_ascii_lowercase() { (((c as u8 - b'a' + k) % 26) + b'a') as char }
        else if c.is_ascii_uppercase() { (((c as u8 - b'A' + k) % 26) + b'A') as char }
        else { c }
    }).collect()
}

fn main() {
    let samples = ['a', 'Z', '7', ' ', '\t', 'é', 'ß', 'Σ', 'σ', 'ς', '中', '😀', '\u{0}', '_', 'Ⅻ', '٣', 'ǅ', '\u{301}'];
    for c in samples {
        println!("{:?} U+{:04X} len8={} len16={} alpha={} num={} alnum={} ws={} up={} low={} ctrl={} punct={} digit10={:?} digit16={:?}",
            c, c as u32, c.len_utf8(), c.len_utf16(), c.is_alphabetic(), c.is_numeric(), c.is_alphanumeric(), c.is_whitespace(),
            c.is_uppercase(), c.is_lowercase(), c.is_control(), c.is_ascii_punctuation(), c.to_digit(10), c.to_digit(16));
    }
    for c in ['a', 'ß', 'Σ', 'ǅ', 'İ', 'ﬃ', 'ŉ'] {
        let up: String = c.to_uppercase().collect();
        let lo: String = c.to_lowercase().collect();
        println!("{} upper={} ({}) lower={} ({})", c, up, c.to_uppercase().len(), lo, lo.chars().count());
    }
    let s = "Grüße, Jürgen! ΣΊΣΥΦΟΣ — 東京 🚀🚀 ok";
    println!("bytes={} chars={} upper={} lower={}", s.len(), s.chars().count(), s.to_uppercase(), s.to_lowercase());
    println!("ascii_up={} ", s.to_ascii_uppercase());
    for (i, c) in s.char_indices().filter(|(_, c)| !c.is_ascii()).take(6) {
        print!("{}:{} ", i, c);
    }
    println!();
    println!("rev={}", s.chars().rev().collect::<String>());
    let enc: Vec<u16> = "a😀".encode_utf16().collect();
    println!("utf16={:x?} back={:?}", enc, String::from_utf16(&enc));
    let mut buf = [0u8; 4];
    let e = '€'.encode_utf8(&mut buf);
    println!("euro bytes {:?} str={}", e.as_bytes(), e);
    println!("from_u32 {:?} {:?} {:?}", char::from_u32(0x41), char::from_u32(0xD800), char::from_u32(0x1F600));
    println!("from_digit {:?} {:?} {:?}", char::from_digit(7, 10), char::from_digit(11, 16), char::from_digit(10, 10));
    println!("escape: {} | {} | {}", "a\"b\\c\n☃".escape_default(), "tab\t\u{7f}é".escape_debug(), "é😀".escape_unicode());
    println!("char escape: {} {} {}", '\''.escape_default(), 'é'.escape_unicode(), '\r'.escape_debug());
    println!("{}", caesar("Hello, World! xyz", 3));
    println!("{}", caesar(&caesar("Round Trip", 13), 13));
    let word_caps: String = "the quick  brown fox".split(' ').map(|w| {
        let mut cs = w.chars();
        match cs.next() { Some(f) => f.to_uppercase().chain(cs).collect::<String>(), None => String::new() }
    }).collect::<Vec<_>>().join(" ");
    println!("[{}]", word_caps);
    let digits: u32 = "a1b2c3x9".chars().filter_map(|c| c.to_digit(10)).sum();
    println!("digit sum {}", digits);
    println!("cmp {} {} {:?}", 'a' < 'b', 'Z' < 'a', 'é'.cmp(&'z'));
    let range: String = ('a'..='e').chain('X'..'Z').collect();
    println!("range {}", range);
    println!("is_char_boundary {} {} {}", s.is_char_boundary(3), s.is_char_boundary(4), s.is_char_boundary(s.len()));
    let bytes = "naïve".as_bytes();
    println!("{:?} {:?}", std::str::from_utf8(bytes), std::str::from_utf8(&bytes[..3]).map_err(|e| (e.valid_up_to(), e.error_len())));
    println!("{}", String::from_utf8_lossy(&[72, 105, 0xff, 0xfe, 33]));
    println!("eq_ignore_ascii_case {} {}", "HeLLo".eq_ignore_ascii_case("hello"), "Straße".eq_ignore_ascii_case("STRASSE"));
    let mut ms = String::from("mixed Case ÄÖ");
    ms.make_ascii_lowercase();
    println!("{}", ms);
    println!("{:?}", "ab\u{301}c".chars().map(|c| c as u32).collect::<Vec<_>>());
    println!("{}", 'x' as u8 + 1);
    println!("{}", ('a' as u8 + 25) as char);
    println!("{:?}", "  \u{a0}trim me\u{2003} ".trim());
    println!("{:?}", "١٢٣".chars().map(|c| c.is_numeric()).collect::<Vec<_>>());
    println!("{:?}", "١٢٣".parse::<i32>().is_err());
}
