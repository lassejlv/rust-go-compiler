// escape_debug of grapheme extenders / non-printable chars inside strings and
// the final-sigma rule of to_lowercase (Case_Ignorable / Cased properties)
// follow std's Unicode tables.
fn h(acc: u64, v: u64) -> u64 {
    (acc ^ v).wrapping_mul(0x100000001b3)
}

fn main() {
    // chars escaped by str::escape_debug when not first (non-printable)
    let mut acc = 0u64;
    let mut n = 0;
    for cp in 0..0x3000u32 {
        let Some(c) = char::from_u32(cp) else { continue };
        let s = format!("a{}", c);
        if s.escape_debug().to_string() != s {
            acc = h(acc, cp as u64);
            n += 1;
        }
    }
    println!("{} {}", n, acc);
    for c in ['\u{34f}', '\u{5bf}', '\u{670}', '\u{fe0f}', '\u{200d}', '\u{e0001}', '\u{1f3fb}', '\u{ad}'] {
        println!("{:?} {} {}", c, format!("a{}", c).escape_debug(), c.escape_debug());
    }
    println!("{:?}", "\u{301}a\u{301}\u{200d}\u{fe0f}\u{10ffff}\u{61c}");
    // final sigma: which chars are skipped (case-ignorable) / count as cased
    let mut acc = [0u64; 3];
    for cp in 0..0x3000u32 {
        let Some(c) = char::from_u32(cp) else { continue };
        let tests = [format!("A{}Σ", c), format!("AΣ{}", c), format!("AΣ{}B", c)];
        for (i, t) in tests.iter().enumerate() {
            if t.to_lowercase().contains('ς') {
                acc[i] = h(acc[i], cp as u64);
            }
        }
    }
    println!("{:?}", acc);
    for s in ["ΑΣ'", "ΑΣ\u{2019}Α", "ǅΣ", "AΣ\u{5bf}", "AΣ\u{a8} ", "Aʰ Σ", "AʰΣ", "ὈΔΥΣΣΕΎΣ"] {
        println!("{} -> {}", s, s.to_lowercase());
    }
}
