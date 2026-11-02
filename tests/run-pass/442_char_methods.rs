// char API: classification (alphabetic, numeric, alphanumeric, whitespace, control, punctuation,
// graphic, hexdigit), case mapping iterators, escape_default/escape_debug/escape_unicode,
// as u32 / from_u32 / from(u8), char ranges and arithmetic, is_ascii_* family, and a classifier table.
fn flags(c: char) -> String {
    let mut f = String::new();
    let checks: [(bool, char); 10] = [
        (c.is_alphabetic(), 'A'),
        (c.is_numeric(), 'N'),
        (c.is_alphanumeric(), 'a'),
        (c.is_whitespace(), 'W'),
        (c.is_control(), 'C'),
        (c.is_lowercase(), 'l'),
        (c.is_uppercase(), 'u'),
        (c.is_ascii_punctuation(), 'P'),
        (c.is_ascii_graphic(), 'G'),
        (c.is_ascii_hexdigit(), 'X'),
    ];
    for (b, ch) in checks {
        f.push(if b { ch } else { '.' });
    }
    f
}

fn main() {
    let chars = ['a', 'Z', '5', ' ', '\n', '\t', '!', '~', 'é', 'Ω', 'ß', '٣', 'Ⅻ', '\u{a0}', '\u{7f}', '中', '🦀', '_', 'F', '\0'];
    for c in chars {
        println!("{:>12} U+{:05X} {}", format!("{:?}", c), c as u32, flags(c));
    }

    println!("escapes:");
    for c in ['a', '\'', '"', '\\', '\n', '\r', '\t', '\0', 'é', '\u{301}', '\u{200b}', '🦀', '\u{7f}'] {
        println!(
            "{:?}: default={} debug={} unicode={}",
            c,
            c.escape_default(),
            c.escape_debug(),
            c.escape_unicode()
        );
    }
    println!("{}", "tab\there \"q\" é\u{1}".escape_default());
    println!("{}", "tab\there \"q\" é\u{1}".escape_debug());
    println!("{}", "ab".escape_unicode());

    println!("case:");
    for c in ['a', 'A', 'ß', 'ǅ', 'İ', 'ﬃ', 'Σ', '1'] {
        let up: String = c.to_uppercase().collect();
        let lo: String = c.to_lowercase().collect();
        println!("{} up={} ({}) lo={} ({}) ascii_up={}", c, up, c.to_uppercase().len(), lo, c.to_lowercase().len(), c.to_ascii_uppercase());
    }

    println!("conversions:");
    println!("{} {} {}", 'A' as u8, 'é' as u8, '🦀' as u16);
    println!("{} {} {:?}", char::from(65u8), char::from(233u8), char::from_u32(0x41));
    println!("{:?} {:?}", u8::try_from('é'), u8::try_from('ÿ'));
    println!("{:?}", char::try_from(0x1F600u32));
    println!("{:?}", char::try_from(0xDFFFu32).is_err());
    let next = |c: char| char::from_u32(c as u32 + 1);
    println!("{:?} {:?} {:?}", next('a'), next('z'), next('\u{D7FF}'));
    println!("{:?} {:?}", ('a'..'f').collect::<String>(), ('x'..='z').rev().collect::<String>());
    println!("{}", ('a'..='z').step_by(5).collect::<String>());
    println!("{}", ('\u{D7FE}'..='\u{E001}').count());
    println!("{:?}", char::MAX);
    println!("{} {}", 'a' < 'b', 'Z' < 'a');
    let mut cs = ['d', 'A', 'z', 'é', '0'];
    cs.sort();
    println!("{:?}", cs);

    println!("ascii:");
    for b in [b'a', b'Q', b'7', b' ', b'\x07', b'#', 0xC3] {
        println!(
            "{:#04x}: alpha={} digit={} ws={} ctrl={} punct={} upper={} ascii={} -> {:?}",
            b,
            b.is_ascii_alphabetic(),
            b.is_ascii_digit(),
            b.is_ascii_whitespace(),
            b.is_ascii_control(),
            b.is_ascii_punctuation(),
            b.to_ascii_uppercase() as char,
            b.is_ascii(),
            b.escape_ascii().to_string()
        );
    }
    println!("{}", b"hi\n\x00\xff".escape_ascii());
    let digits: u32 = "a1b2c3".chars().filter_map(|c| c.to_digit(10)).sum();
    println!("digit sum {}", digits);
    let counts = "Hello, World! 123".chars().fold([0usize; 4], |mut acc, c| {
        let i = if c.is_alphabetic() { 0 } else if c.is_numeric() { 1 } else if c.is_whitespace() { 2 } else { 3 };
        acc[i] += 1;
        acc
    });
    println!("{:?}", counts);
}
