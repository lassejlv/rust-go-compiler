// char escapes, UTF-8 encoding/decoding, boundaries, escape_debug/default/unicode
fn main() {
    let s = "héllo wörld ✓ 日本 🎉";
    println!("{} bytes, {} chars", s.len(), s.chars().count());
    for (i, c) in s.char_indices().filter(|(_, c)| !c.is_ascii()) {
        println!("{:>2} {:?} len_utf8={} len_utf16={} U+{:04X}", i, c, c.len_utf8(), c.len_utf16(), c as u32);
    }
    println!("{}", (0..=s.len()).filter(|&i| s.is_char_boundary(i)).count());
    println!("{:?}", s.get(0..2));
    println!("{:?}", s.get(0..3));
    println!("{:?} {:?}", s.get(1..), s.get(100..));
    let mut buf = [0u8; 4];
    for c in ['a', 'é', '✓', '🎉'] {
        let enc = c.encode_utf8(&mut buf);
        println!("{} -> {:?} {:x?}", c, enc.len(), enc.as_bytes());
    }
    let mut u16buf = [0u16; 2];
    println!("{:x?}", '🎉'.encode_utf16(&mut u16buf));
    let utf16: Vec<u16> = "a🎉b".encode_utf16().collect();
    println!("{:x?} {:?}", utf16, String::from_utf16(&utf16));
    println!("{:?}", String::from_utf16(&[0xD800, 0x61]).is_err());
    println!("{}", String::from_utf16_lossy(&[0x61, 0xDC00, 0x62]));
    for c in ['\n', '\t', '\'', '"', '\\', '\0', '\u{7f}', 'é', '\u{200b}', '\u{301}'] {
        println!("{:?} default={} debug={} unicode={}", c, c.escape_default(), c.escape_debug(), c.escape_unicode());
    }
    println!("{}", "tab\there\n\"q\"".escape_default());
    println!("{}", "é\u{301}\r".escape_debug());
    println!("{:?}", "mixed\u{1b}[0m\u{feff}");
    println!("{:?}", "\u{e9}\u{301}");
    println!("{:?}", char::from_u32(0x1F600));
    println!("{:?} {:?}", char::from_u32(0xD800), char::from_u32(0x110000));
    println!("{:?}", char::from_digit(11, 16));
    println!("{:?} {:?}", 'z'.to_digit(36), 'Z'.to_digit(10));
    println!("{}", ('a'..='e').rev().collect::<String>());
    println!("{:?}", ('α'..'ε').collect::<Vec<char>>());
    println!("{}", 'ß'.to_uppercase());
    println!("{}", 'İ'.to_lowercase().count());
    println!("{} {}", 'ǅ'.is_uppercase(), 'ǅ'.is_lowercase());
    let bytes = s.as_bytes();
    println!("{:?}", std::str::from_utf8(&bytes[..2]));
    println!("{:?}", std::str::from_utf8(&bytes[..3]).map(|s| s.len()));
    let err = std::str::from_utf8(&bytes[..2]).unwrap_err();
    println!("{} valid_up_to={} {:?}", err, err.valid_up_to(), err.error_len());
    let bad = [0x66, 0x6f, 0xff, 0x6f];
    let e2 = std::str::from_utf8(&bad).unwrap_err();
    println!("{} {:?}", e2, e2.error_len());
    println!("{}", String::from_utf8_lossy(&bad));
    println!("{}", String::from_utf8_lossy(&[0xe6, 0x97, 0xa5, 0xe6, 0x97]));
    let rev: String = s.chars().rev().collect();
    println!("{}", rev);
    let truncated: String = s.chars().take(7).collect();
    println!("{}", truncated);
    let mut idx = s.len() / 2;
    while !s.is_char_boundary(idx) {
        idx -= 1;
    }
    println!("{} | {}", &s[..idx], &s[idx..]);
    println!("{:?}", s.split_at_checked(2));
    println!("{:?}", s.bytes().filter(|b| *b >= 0x80).count());
    let ascii = "Hello";
    println!("{} {}", ascii.is_ascii(), s.is_ascii());
    println!("{:?}", "AbC".chars().map(|c| c.is_ascii_uppercase()).collect::<Vec<_>>());
    println!("{}", 'A' as u8 + 2);
    println!("{}", (b'a' + 25) as char);
    println!("{}", '\u{41}');
    println!("{:?}", "x\u{0}y".len());
    let samples = ["a\u{0}b", "tab\t", "quote\"", "uni\u{1F600}", "ctrl\u{1}", "del\u{7f}", "nbsp\u{a0}", "zwj\u{200d}x"];
    for s in samples {
        println!("{:?} {} {}", s, s.escape_debug(), s.escape_default());
    }
    let grapheme_like = "e\u{301}a\u{308}";
    println!("{} chars={} bytes={}", grapheme_like, grapheme_like.chars().count(), grapheme_like.len());
    let rev: String = grapheme_like.chars().rev().collect();
    println!("{:?}", rev);
    let mut counts = [0usize; 5];
    for c in "aé✓🎉".chars() {
        counts[c.len_utf8()] += 1;
    }
    println!("{:?}", counts);
    let surrogate_free: Vec<u32> = "𝄞x".chars().map(|c| c as u32).collect();
    println!("{:x?}", surrogate_free);
    println!("{:?}", char::decode_utf16([0xD834u16, 0xDD1E, 0xD800, 0x0061]).map(|r| r.map_err(|e| e.unpaired_surrogate())).collect::<Vec<_>>());
    println!("{}", '\u{10FFFF}'.len_utf8());
    println!("{:?}", ('a'..='z').step_by(5).collect::<String>());
    println!("{}", char::MAX as u32);
}
