fn main() {
    let s = "héllo wörld ß 日本語 🦀!";
    println!("bytes {} chars {}", s.len(), s.chars().count());
    for (i, c) in s.char_indices().filter(|(_, c)| !c.is_ascii()) {
        println!("{} {:?} len_utf8={} u+{:04X}", i, c, c.len_utf8(), c as u32);
    }
    let rev: String = s.chars().rev().collect();
    println!("{}", rev);
    println!("{}", s.to_uppercase());
    println!("{}", "STRASSE Straße".to_lowercase());
    println!("{:?}", 'ß'.to_uppercase().collect::<String>());
    println!("{}", 'ß'.to_uppercase().count());
    println!("{:?}", 'İ'.to_lowercase().collect::<String>());
    println!("{} {}", 'ǅ'.is_uppercase(), 'ǅ'.is_lowercase());
    let (left, right) = s.split_at(7);
    println!("[{}] [{}]", left, right);
    println!("{} {}", s.is_char_boundary(2), s.is_char_boundary(3));
    println!("{:?}", s.get(1..3));
    println!("{:?}", s.get(1..2));
    let crab_pos = s.find('🦀').unwrap();
    println!("crab at {} -> {:?}", crab_pos, &s[crab_pos..]);
    println!("{:?}", "日本語".bytes().collect::<Vec<u8>>());
    println!("{:?}", "añb".as_bytes());
    println!("{:?}", std::str::from_utf8(&[0xe6, 0x97, 0xa5]));
    println!("{}", std::str::from_utf8(&[0xff, 0x00]).is_err());
    println!("{}", String::from_utf8_lossy(&[72, 105, 0xff, 33]));
    println!("|{:>10}|{:<8}|{:^9}|", "日本", "ñu", "🦀");
    println!("|{:*>6}|{:-<6}|", "é", "ß");
    println!("|{:>5.2}|", "héllo");
    let words: Vec<&str> = "α,β,,γ".split(',').collect();
    println!("{:?}", words);
    println!("{:?}", "  ÄÖÜ  ".trim());
    println!("{}", "über".replace('ü', "ue"));
    println!("{:?}", "ab日c".chars().map(|c| c.is_alphabetic()).collect::<Vec<_>>());
    println!("{:?}", "x²³".chars().map(|c| (c.is_numeric(), c.is_ascii_digit())).collect::<Vec<_>>());
    println!("{:?}", char::from_u32(0x1F600));
    println!("{:?}", char::from_u32(0xD800));
    println!("{:?}", char::from_digit(7, 10));
    println!("{:?}", 'é'.escape_unicode().to_string());
    println!("{}", "tab\there\n".escape_debug());
    println!("{}", "日".escape_default());
    let mut owned = String::from("añ");
    owned.push('🦀');
    owned.insert(0, 'ü');
    let popped = owned.pop();
    println!("{} {} {:?}", owned, owned.len(), popped);
    owned.truncate(3);
    println!("{:?}", owned);
    let encoded: Vec<u16> = "a🦀".encode_utf16().collect();
    println!("{:?}", encoded);
    println!("{:?}", String::from_utf16(&encoded));
    let mut cs: Vec<char> = "dcbaé".chars().collect();
    cs.sort();
    println!("{:?}", cs);
    println!("{}", "Hello".chars().all(char::is_alphanumeric));
    println!("{}", "hello wörld".split(' ').map(|w| {
        let mut c = w.chars();
        match c.next() {
            Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
            None => String::new(),
        }
    }).collect::<Vec<_>>().join(" "));
}
