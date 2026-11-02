// Unicode string handling: chars vs bytes vs char_indices, len_utf8/len_utf16, encode_utf8,
// is_char_boundary, slicing at boundaries, from_utf8 errors, from_utf8_lossy, utf16 round trip,
// reversing by chars, counting categories, and truncating safely to a byte budget.
fn truncate_bytes(s: &str, max: usize) -> &str {
    if s.len() <= max {
        return s;
    }
    let mut end = max;
    while !s.is_char_boundary(end) {
        end -= 1;
    }
    &s[..end]
}

fn categories(s: &str) -> (usize, usize, usize, usize, usize) {
    let mut a = (0, 0, 0, 0, 0);
    for c in s.chars() {
        if c.is_alphabetic() {
            a.0 += 1;
        }
        if c.is_numeric() {
            a.1 += 1;
        }
        if c.is_whitespace() {
            a.2 += 1;
        }
        if c.is_ascii() {
            a.3 += 1;
        }
        if c.is_uppercase() {
            a.4 += 1;
        }
    }
    a
}

fn main() {
    let samples = ["hello", "héllo", "日本語", "🦀 rust", "ÅΩ\u{301}", "a\u{0}b", ""];
    for s in samples {
        println!(
            "{:?}: bytes={} chars={} utf16={} first={:?} last={:?}",
            s,
            s.len(),
            s.chars().count(),
            s.encode_utf16().count(),
            s.chars().next(),
            s.chars().next_back()
        );
    }

    let s = "aé日🦀";
    for (i, c) in s.char_indices() {
        let mut buf = [0u8; 4];
        let enc = c.encode_utf8(&mut buf);
        println!("{} {:?} U+{:04X} utf8={} utf16={} bytes={:02x?}", i, c, c as u32, c.len_utf8(), c.len_utf16(), enc.as_bytes());
    }
    let boundaries: Vec<usize> = (0..=s.len()).filter(|&i| s.is_char_boundary(i)).collect();
    println!("boundaries {:?}", boundaries);
    println!("{:?} {:?} {:?}", &s[1..3], s.get(1..2), s.get(3..7));

    for max in [0, 1, 2, 3, 4, 6, 7, 10, 20] {
        println!("trunc {} -> {:?}", max, truncate_bytes(s, max));
    }

    let rev: String = "añb日".chars().rev().collect();
    println!("rev {}", rev);
    let utf16: Vec<u16> = "a🦀".encode_utf16().collect();
    println!("utf16 {:04x?} back={:?}", utf16, String::from_utf16(&utf16));
    println!("{:?}", String::from_utf16(&[0xd800]).is_err());
    let lossy16 = String::from_utf16_lossy(&[0x61, 0xdc00, 0x62]);
    println!("{:?}", lossy16);

    let bad = vec![0x66, 0x6f, 0xff, 0x6f, 0xe6, 0x97];
    match std::str::from_utf8(&bad) {
        Ok(s) => println!("ok {}", s),
        Err(e) => println!("utf8 error valid_up_to={} len={:?}", e.valid_up_to(), e.error_len()),
    }
    let tail = &bad[4..];
    println!("{:?}", std::str::from_utf8(tail).map_err(|e| e.error_len()));
    println!("lossy {:?}", String::from_utf8_lossy(&bad));
    println!("{:?}", String::from_utf8(vec![0xce, 0xbb]).unwrap());

    let text = "Ünïcödé Täxt 123 ٤٥ ⅷ  done";
    println!("{:?}", categories(text));
    let words: Vec<&str> = text.split_whitespace().collect();
    println!("{:?}", words);
    let caps: Vec<String> = words
        .iter()
        .map(|w| {
            let mut cs = w.chars();
            match cs.next() {
                Some(f) => f.to_uppercase().collect::<String>() + &cs.as_str().to_lowercase(),
                None => String::new(),
            }
        })
        .collect();
    println!("{:?}", caps);

    let bytes: Vec<u8> = "€".bytes().collect();
    println!("{:?} {}", bytes, bytes.iter().map(|b| format!("{:08b}", b)).collect::<Vec<_>>().join(" "));
    println!("{:?}", char::from_u32(0x1F980));
    println!("{:?} {:?}", char::from_u32(0xD800), char::from_u32(0x110000));
    println!("{}", ('a'..='e').chain('α'..='δ').collect::<String>());
    let c = 'ß';
    println!("{} {} {}", c.to_uppercase(), c.to_uppercase().count(), 'ǅ'.to_lowercase());
    println!("{:?}", "ﬁ".to_uppercase());
    let mut owned = String::from("héllo wörld");
    owned.insert(0, '¡');
    owned.insert_str(owned.len(), "!");
    let removed = owned.remove(2);
    println!("{} removed={:?} len={}", owned, removed, owned.len());
    owned.retain(|c| c != 'l');
    println!("{}", owned);
    let pos = owned.find('w').unwrap();
    let (a, b) = owned.split_at(pos);
    println!("{:?} {:?}", a, b);
    owned.truncate(owned.char_indices().nth(3).map(|(i, _)| i).unwrap());
    println!("{:?} pop={:?} {:?}", owned.clone(), owned.pop(), owned);
}
