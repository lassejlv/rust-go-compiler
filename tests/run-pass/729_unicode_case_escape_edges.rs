// Unicode edge cases: multi-char case mappings, final sigma, Debug escaping of
// grapheme extenders / non-printables, char classification, char boundaries,
// and UTF-8/UTF-16 lengths.
fn classify(c: char) -> String {
    let mut f = Vec::new();
    if c.is_alphabetic() { f.push("alpha"); }
    if c.is_numeric() { f.push("num"); }
    if c.is_alphanumeric() { f.push("alnum"); }
    if c.is_whitespace() { f.push("ws"); }
    if c.is_uppercase() { f.push("upper"); }
    if c.is_lowercase() { f.push("lower"); }
    if c.is_control() { f.push("ctrl"); }
    if c.is_ascii_punctuation() { f.push("punct"); }
    f.join(",")
}

fn main() {
    let words = ["straße", "İstanbul", "ΟΔΟΣ ΣΟΦΟΣ", "ǅungla", "ﬁne", "Ωmega", "ὈΔΥΣΣΕΎΣ", "groß", "ΣΑΣ.", "aΣb"];
    for w in words {
        println!("{:<12} upper={:<14} lower={:<14} chars={} bytes={}", w, w.to_uppercase(), w.to_lowercase(), w.chars().count(), w.len());
    }
    for c in ['ß', 'ǅ', 'ﬃ', 'İ', 'ŉ', 'a', 'Σ', 'ς'] {
        let up: String = c.to_uppercase().collect();
        let lo: String = c.to_lowercase().collect();
        println!("{:?}: upper {:?} ({}) lower {:?} ({}) len_utf8={} len_utf16={}", c, up, c.to_uppercase().len(), lo, c.to_lowercase().len(), c.len_utf8(), c.len_utf16());
    }
    let tricky = ["e\u{301}", "\u{200b}zero", "tab\there", "bell\u{7}", "\u{feff}bom", "nbsp\u{a0}x", "emoji 🦀", "\u{10ffff}", "a\u{300}\u{301}", "\r\n", "'\"", "\u{1f}", "del\u{7f}", "soft\u{ad}hy", "\u{e000}pua"];
    for t in tricky {
        println!("{:?} esc_default={} esc_debug={}", t, t.escape_default(), t.escape_debug());
    }
    for c in ['\u{301}', '\u{200d}', '\'', '"', '\u{0}', 'é', '\u{85}'] {
        println!("char {:?} esc_unicode={} esc_default={}", c, c.escape_unicode(), c.escape_default());
    }
    for c in ['a', 'Z', '5', '½', '٣', '漢', ' ', '\u{a0}', '\u{2003}', '\u{85}', '\n', '_', '!', 'Ⅻ', 'ǅ', 'ª'] {
        println!("{:?}: [{}] to_digit10={:?} to_digit16={:?}", c, classify(c), c.to_digit(10), c.to_digit(16));
    }
    let s = "a\u{a0}b\u{2003}c\u{3000}d\te\u{85}f";
    println!("split_whitespace: {:?}", s.split_whitespace().collect::<Vec<_>>());
    println!("split_ascii_whitespace: {:?}", s.split_ascii_whitespace().collect::<Vec<_>>());
    let mixed = "héllo wörld ✓";
    let boundaries: Vec<usize> = (0..=mixed.len()).filter(|&i| mixed.is_char_boundary(i)).collect();
    println!("boundaries: {:?}", boundaries);
    println!("char_indices: {:?}", mixed.char_indices().filter(|(_, c)| !c.is_ascii()).collect::<Vec<_>>());
    println!("rev chars: {}", mixed.chars().rev().collect::<String>());
    println!("utf16 len: {} encode: {:?}", mixed.encode_utf16().count(), "𝄞a".encode_utf16().collect::<Vec<_>>());
    println!("from_u32: {:?} {:?} {:?}", char::from_u32(0x1F980), char::from_u32(0xDFFF), char::from_u32(0x110000));
    println!("from_digit: {:?} {:?} {:?}", char::from_digit(7, 10), char::from_digit(11, 16), char::from_digit(10, 10));
    println!("eq_ignore_ascii_case: {} {}", "HeLLo".eq_ignore_ascii_case("hello"), "Straße".eq_ignore_ascii_case("STRASSE"));
    println!("ascii upper only: {}", "héllo straße".to_ascii_uppercase());
    let mut owned = String::from("ﬁx Σ");
    owned.make_ascii_lowercase();
    println!("make_ascii_lowercase: {}", owned);
    println!("trim unicode: {:?}", "\u{3000}\u{a0} padded \u{2009}".trim());
    println!("max char: {:?} {}", char::MAX, char::MAX as u32);
    let cmp: Vec<_> = ["b", "B", "á", "a", "Z", "ä"].iter().map(|s| s.to_string()).collect::<std::collections::BTreeSet<_>>().into_iter().collect();
    println!("byte order: {:?}", cmp);
}
