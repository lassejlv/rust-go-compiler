fn main() {
    let s = "Straße groß ǅ ﬃ İstanbul";
    println!("{}", s.to_uppercase());
    println!("{}", s.to_lowercase());
    for c in ['ß', 'ﬃ', 'İ', 'ǅ', 'a', 'Σ', '1'] {
        let up: String = c.to_uppercase().collect();
        let lo: String = c.to_lowercase().collect();
        println!("{} -> up {:?} ({}) lo {:?} ({})", c, up, c.to_uppercase().len(), lo, c.to_lowercase().count());
    }
    println!("{}", "ὈΔΥΣΣΕΎΣ".to_lowercase());
    let t = "héllo wörld";
    for (i, c) in t.char_indices().filter(|(_, c)| !c.is_ascii()) {
        println!("{} {} {}", i, c, c.len_utf8());
    }
    println!("{} {}", t.len(), t.chars().count());
    println!("{} {} {}", t.is_char_boundary(1), t.is_char_boundary(2), t.is_char_boundary(3));
    println!("{:?} {:?}", t.get(0..2), t.get(0..3));
    println!("{:?}", t.split_once(' '));
    println!("{:?}", t.rsplit_once('l'));
    println!("{:?}", "k=v=w".split_once('='));
    println!("{:?}", "none".split_once(':'));
    println!("{:?}", "a::b::c".split_once("::"));
    println!("{}", "xxabcxx".trim_matches('x'));
    println!("{}", "123abc456".trim_matches(|c: char| c.is_numeric()));
    println!("{}", "123abc456".trim_start_matches(char::is_numeric));
    println!("{}", "abab__abab".trim_end_matches("ab"));
    println!("{}", "  \t pad \n".trim());
    println!("{:?} {:?}", "prefix_rest".strip_prefix("prefix_"), "x".strip_suffix("y"));
    println!("{:?}", "a1b2c3".matches(char::is_numeric).collect::<Vec<_>>());
    println!("{:?}", "abcabca".match_indices("ca").collect::<Vec<_>>());
    println!("{:?}", "abcabca".rmatch_indices('a').collect::<Vec<_>>());
    println!("{:?} {:?}", "hello".find('l'), "hello".rfind('l'));
    println!("{:?} {:?}", "hello".find("lo"), "hello".find(|c: char| c > 'h'));
    println!("{:?}", "hello".find(['o', 'e']));
    println!("{:?}", "a,b;c d".split([',', ';', ' ']).collect::<Vec<_>>());
    println!("{:?}", "a,b;c".split(&[',', ';'][..]).collect::<Vec<_>>());
    println!("{}", "aaa".replace("a", "bb"));
    println!("{}", "aaa".replacen('a', "c", 2));
    println!("{}", "Hello".repeat(2));
    let mut st = String::from("hello");
    st.insert(0, '¡');
    st.insert_str(st.len(), " world");
    println!("{} {}", st, st.len());
    println!("{:?} {:?}", st.pop(), st.remove(0));
    st.truncate(5);
    println!("{}", st);
    st.retain(|c| c != 'l');
    println!("{}", st);
    let mut s2 = String::from("abcdef");
    let dr: String = s2.drain(1..4).collect();
    println!("{} {}", dr, s2);
    s2.replace_range(0..1, "XYZ");
    println!("{}", s2);
    let tail = s2.split_off(2);
    println!("{} {}", s2, tail);
    println!("{:?}", "line1\nline2\r\nline3\n".lines().collect::<Vec<_>>());
    println!("{:?}", "a\n\nb".split('\n').collect::<Vec<_>>());
    println!("{:?}", "ab".bytes().rev().collect::<Vec<u8>>());
    println!("{:?}", String::from_utf8(vec![0xe2, 0x82, 0xac]));
    println!("{:?}", String::from_utf8(vec![0xff, 0x41]).is_err());
    println!("{}", String::from_utf8_lossy(&[0x41, 0xff, 0x42]));
    println!("{:?}", std::str::from_utf8(&[0xce, 0xbb]));
    println!("{:?}", "€".as_bytes());
    let cs: Vec<char> = "añ€😀".chars().collect();
    println!("{:?}", cs.iter().map(|c| c.len_utf8()).collect::<Vec<_>>());
    println!("{:?}", cs.iter().map(|c| c.len_utf16()).collect::<Vec<_>>());
    println!("{:?}", "😀".encode_utf16().collect::<Vec<u16>>());
    println!("{:?}", String::from_utf16(&[0xd83d, 0xde00, 0x61]));
    let mut buf = [0u8; 4];
    println!("{:?}", 'ñ'.encode_utf8(&mut buf).as_bytes());
    println!("{} {} {}", 'z' as u32 - 'a' as u32, ('a' as u8 + 2) as char, char::from(66u8));
    println!("{:?} {:?}", char::from_digit(11, 16), 'F'.to_digit(16));
    println!("{:?}", 'F'.to_digit(10));
    println!("{} {} {} {}", '9'.is_alphanumeric(), ' '.is_whitespace(), 'é'.is_alphabetic(), 'A'.is_lowercase());
    println!("{} {}", 'a'.to_ascii_uppercase(), "MiXeD".to_ascii_lowercase());
    println!("{} {}", '\u{a0}'.is_whitespace(), 'Ⅻ'.is_numeric());
    println!("{}", "Hello World".chars().filter(|c| c.is_uppercase()).collect::<String>());
    println!("{}", "abc".chars().rev().collect::<String>());
    println!("{:?}", "abc".char_indices().rev().next());
    println!("{} {}", "apple" < "banana", "Zebra" < "apple");
    println!("{:?}", "b".cmp("abc"));
    println!("{}", "tschüß".eq_ignore_ascii_case("TSCHüß"));
    let words = "the quick brown fox";
    let caps: Vec<String> = words
        .split(' ')
        .map(|w| {
            let mut c = w.chars();
            match c.next() {
                Some(f) => f.to_uppercase().chain(c).collect(),
                None => String::new(),
            }
        })
        .collect();
    println!("{}", caps.join(" "));
    println!("{}", 'ß'.is_lowercase());
    println!("{:?}", "x".parse::<char>());
    println!("{:?}", "xy".parse::<char>().is_err());
    println!("{:?}", "true".parse::<bool>());
    println!("{}", "ab".contains("b") && "ab".contains('a') && !"ab".contains(char::is_whitespace));
    println!("{:?}", "a-b_c".split(|c| c == '-' || c == '_').collect::<Vec<_>>());
    println!("{:?}", "1 2 3".split(' ').map(str::parse::<i32>).collect::<Result<Vec<_>, _>>());
    println!("{}", format!("{}{}", "a", 'b') + &String::from("c") + "d");
    let mut owned = String::new();
    owned += "x";
    owned.push('y');
    owned.extend(['z', 'w'].iter());
    owned.extend(vec!["!", "?"]);
    println!("{} {}", owned, owned.capacity() >= owned.len());
    println!("{:?}", "AbC".char_indices().map(|(i, c)| if i % 2 == 0 { c.to_ascii_lowercase() } else { c }).collect::<String>());
    println!("{:?}", "ab cd".split_ascii_whitespace().collect::<Vec<_>>());
    println!("{}", "abc".escape_default());
    println!("{}", "a\tb\u{301}".escape_debug());
    println!("{}", "é\n".escape_unicode());
    println!("{:?}", "hello"[1..].chars().next());
    println!("{:?}", "αβγ".char_indices().nth(2));
    println!("{}", &"αβγ"[2..4]);
}
