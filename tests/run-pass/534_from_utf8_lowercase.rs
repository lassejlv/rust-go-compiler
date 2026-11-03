// String::from_utf8 errors, into_bytes, utf8_chunks, case conversion of non-ASCII, char case mapping
fn main() {
    let good = vec![0x68, 0x69, 0xe2, 0x9c, 0x93];
    let s = String::from_utf8(good.clone()).unwrap();
    println!("{} {}", s, s.len());
    let bad = vec![0x61, 0x62, 0xc3, 0x28, 0x63];
    match String::from_utf8(bad) {
        Ok(_) => println!("unexpected"),
        Err(e) => {
            println!("err: {}", e);
            println!("valid_up_to={} error_len={:?}", e.utf8_error().valid_up_to(), e.utf8_error().error_len());
            let bytes = e.into_bytes();
            println!("recovered {:?}", bytes);
        }
    }
    let truncated = vec![0xe2, 0x9c];
    let e = String::from_utf8(truncated).unwrap_err();
    println!("{} {:?}", e, e.utf8_error().error_len());
    println!("{:?}", e.as_bytes());
    let mixed = b"ok\xffbad\xe2\x9c\x93end\xc0";
    for chunk in mixed.utf8_chunks() {
        println!("valid={:?} invalid={:x?}", chunk.valid(), chunk.invalid());
    }
    println!("{}", String::from_utf8_lossy(mixed));
    println!("{:?}", String::from_utf8_lossy(b"plain").into_owned());

    let words = ["HELLO", "Straße", "ΣΊΣΥΦΟΣ", "İstanbul", "ÀÉÎÕÜ", "ǅungla", "ﬁne", "ΌΣΟΣ ΚΑΙ", "Σ"];
    for w in words {
        println!("{} -> lower {} upper {} (len {}->{})", w, w.to_lowercase(), w.to_uppercase(), w.len(), w.to_uppercase().len());
    }
    println!("{}", "ß".to_uppercase());
    println!("{}", "ŉ".to_uppercase());
    for c in ['A', 'ß', 'İ', 'ﬃ', 'Σ', 'ς', 'ǈ', '1', 'é'] {
        let lo: String = c.to_lowercase().collect();
        let up: String = c.to_uppercase().collect();
        println!("{:?}: lower={:?} upper={:?} alpha={} lower?={} upper?={}", c, lo, up, c.is_alphabetic(), c.is_lowercase(), c.is_uppercase());
    }
    let mut ascii = String::from("MiXeD ÄsCiI");
    ascii.make_ascii_lowercase();
    println!("{}", ascii);
    ascii.make_ascii_uppercase();
    println!("{}", ascii);
    println!("{}", "Grüße".to_ascii_uppercase());
    println!("{}", "Grüße".eq_ignore_ascii_case("GRüSSE"));
    println!("{}", "Grüße".to_lowercase() == "grüße");
    let s2 = String::from("héllo");
    let bytes = s2.into_bytes();
    println!("{:?}", bytes);
    let back = String::from_utf8(bytes).unwrap();
    println!("{}", back);
    let mut st = String::with_capacity(8);
    for c in ['a', 'é', '中', '😀'] {
        st.push(c);
        println!("{} len={} chars={}", st, st.len(), st.chars().count());
    }
    println!("{:?}", st.pop());
    st.insert(1, 'X');
    st.insert_str(0, ">>");
    println!("{}", st);
    println!("{:?}", st.remove(2));
    println!("{}", st);
    st.truncate(st.char_indices().nth(2).map(|(i, _)| i).unwrap());
    println!("{:?}", st);
    let mut owned = String::from("a,b,c");
    owned.retain(|c| c != ',');
    println!("{}", owned);
    let dr: String = owned.drain(..1).collect();
    println!("{} {}", dr, owned);
    owned.replace_range(..1, "ZZ");
    println!("{}", owned);
    let u = unsafe { String::from_utf8_unchecked(vec![b'o', b'k']) };
    println!("{}", u);
    let chars: Vec<char> = "αβγ".chars().collect();
    let up: String = chars.iter().flat_map(|c| c.to_uppercase()).collect();
    println!("{}", up);
    let sorted_ci = {
        let mut v = vec!["éclair", "Apple", "zebra", "Éte", "apple"];
        v.sort_by_key(|s| s.to_lowercase());
        v
    };
    println!("{:?}", sorted_ci);
    println!("{:?}", "ΑΒΓ".char_indices().map(|(i, _)| i).collect::<Vec<_>>());
    println!("{}", "tschüß".len());
}
