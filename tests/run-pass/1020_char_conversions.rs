fn main() {
    for v in [0x41u32, 0x1F600, 0xD800, 0x110000, 0x7F, 0xE9] {
        match char::from_u32(v) {
            Some(c) => println!("{:#x} -> {:?} len_utf8={} len_utf16={}", v, c, c.len_utf8(), c.len_utf16()),
            None => println!("{:#x} -> invalid", v),
        }
    }
    for c in ['7', 'a', 'F', 'z', 'g', ' '] {
        println!("{:?}: d10={:?} d16={:?} d36={:?}", c, c.to_digit(10), c.to_digit(16), c.to_digit(36));
    }
    println!("{:?}", (0..16).map(|d| char::from_digit(d, 16).unwrap()).collect::<String>());
    println!("{:?} {:?}", char::from_digit(10, 10), char::from_digit(35, 36));
    let s = "Grüße, Jürgen ❤ ΣΑΣ";
    println!("{}", s.to_uppercase());
    println!("{}", s.to_lowercase());
    println!("{:?}", 'ß'.to_uppercase().collect::<String>());
    println!("{:?}", 'İ'.to_lowercase().collect::<String>());
    for c in ['a', 'Z', '5', ' ', '\n', 'é', '_', '!', '中'] {
        println!("{:?} alpha={} num={} alnum={} ws={} upper={} lower={} ascii={} punct={} ctrl={}",
            c, c.is_alphabetic(), c.is_numeric(), c.is_alphanumeric(), c.is_whitespace(),
            c.is_uppercase(), c.is_lowercase(), c.is_ascii(), c.is_ascii_punctuation(), c.is_control());
    }
    println!("{}", 'a' as u32 + 1);
    println!("{:?}", (b'a'..=b'e').map(char::from).collect::<Vec<_>>());
    println!("{:?}", ('a'..'f').rev().collect::<String>());
    println!("{:?}", ('x'..='z').count());
    let mut buf = [0u8; 4];
    let enc = '€'.encode_utf8(&mut buf);
    println!("{} {:?}", enc, enc.as_bytes());
    let mut b16 = [0u16; 2];
    println!("{:?}", '😀'.encode_utf16(&mut b16));
    println!("{}", '\u{1F600}'.escape_unicode());
    println!("{} {}", '\t'.escape_default(), '"'.escape_debug());
    println!("{:?}", "tab\there \u{7f} é".escape_default().to_string());
    println!("{}", 'A'.eq_ignore_ascii_case(&'a'));
    let mut c = 'q';
    c.make_ascii_uppercase();
    println!("{} {}", c, 'Q'.to_ascii_lowercase());
    println!("{:?}", char::decode_utf16([0xD83D, 0xDE00, 0x41, 0xDC00]).map(|r| r.map_err(|e| e.unpaired_surrogate())).collect::<Vec<_>>());
    println!("{:?}", u8::try_from('é').is_err());
    println!("{:?}", char::try_from(65u8));
    println!("{:?}", char::try_from(0xD800u32).is_err());
    println!("{}", char::MAX as u32);
    println!("{:?}", "héllo".char_indices().collect::<Vec<_>>());
    println!("{:?}", "a1b22c333".split(|c: char| c.is_ascii_digit()).filter(|s| !s.is_empty()).collect::<Vec<_>>());
}
