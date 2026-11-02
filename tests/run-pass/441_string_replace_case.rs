// String transformations: replace/replacen with str/char/closure patterns, to_uppercase /
// to_lowercase of non-ASCII (ß, Σ final sigma, İ, ŉ), ASCII-only case ops, eq_ignore_ascii_case,
// repeat, char-level caesar/rot13, camel/snake case conversion, and capacity-independent building.
fn snake_to_camel(s: &str) -> String {
    let mut out = String::new();
    let mut up = false;
    for c in s.chars() {
        if c == '_' {
            up = true;
        } else if up {
            out.extend(c.to_uppercase());
            up = false;
        } else {
            out.push(c);
        }
    }
    out
}

fn camel_to_snake(s: &str) -> String {
    let mut out = String::new();
    for (i, c) in s.char_indices() {
        if c.is_uppercase() {
            if i > 0 {
                out.push('_');
            }
            out.extend(c.to_lowercase());
        } else {
            out.push(c);
        }
    }
    out
}

fn rot13(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            'a'..='z' => (((c as u8 - b'a') + 13) % 26 + b'a') as char,
            'A'..='Z' => (((c as u8 - b'A') + 13) % 26 + b'A') as char,
            _ => c,
        })
        .collect()
}

fn title_case(s: &str) -> String {
    s.split(' ')
        .map(|w| {
            let mut c = w.chars();
            match c.next() {
                Some(f) => f.to_uppercase().chain(c.flat_map(|x| x.to_lowercase())).collect(),
                None => String::new(),
            }
        })
        .collect::<Vec<String>>()
        .join(" ")
}

fn main() {
    let s = "the cat sat on the mat";
    println!("{}", s.replace("the", "a"));
    println!("{}", s.replacen("the", "THE", 1));
    println!("{}", s.replace('a', "@"));
    println!("{}", s.replace(|c: char| "aeiou".contains(c), ""));
    println!("{}", s.replace(&['c', 'm'][..], "b"));
    println!("{}", s.replace("", "|"));
    println!("{}", "aaa".replace("aa", "b"));
    println!("{}", "日本日本".replace("日", "月"));
    println!("{}", s.replacen(char::is_whitespace, "_", 3));

    let words = ["straße", "ΣΊΣΥΦΟΣ", "İstanbul", "ŉ", "ǆ", "Ǳ", "ﬀ", "ΌΣΟΣ ΣΑ"];
    for w in words {
        println!("{} -> upper={} lower={} ascii_up={}", w, w.to_uppercase(), w.to_lowercase(), w.to_ascii_uppercase());
    }
    println!("{}", "Hello Wörld".to_ascii_lowercase());
    let mut m = String::from("MiXeD ÄÖ");
    m.make_ascii_lowercase();
    println!("{}", m);
    println!("{} {}", "HeLLo".eq_ignore_ascii_case("hello"), "Ä".eq_ignore_ascii_case("ä"));

    for n in ["parse_http_request", "already", "_leading", "trailing_", "a_b_c"] {
        let c = snake_to_camel(n);
        println!("{} -> {} -> {}", n, c, camel_to_snake(&c));
    }
    println!("{}", camel_to_snake("XMLHttpRequest"));
    println!("{}", rot13("Hello, World!"));
    println!("{}", rot13(&rot13("Round Trip")));
    println!("{}", title_case("the qUICK bRown  fox ǆungla"));

    println!("{}", "ab".repeat(3));
    println!("[{}]", "-".repeat(0));
    let line = format!("+{}+", "=".repeat(10));
    println!("{}", line);

    // building strings with different capacities gives same results
    let mut a = String::new();
    let mut b = String::with_capacity(1000);
    for i in 0..20 {
        let piece = format!("{}{}", i, if i % 3 == 0 { "!" } else { "" });
        a.push_str(&piece);
        b += &piece;
    }
    println!("{} {}", a == b, a);
    a.shrink_to_fit();
    println!("{} {}", a.len(), a == b);
    let mut c = a.clone();
    c.clear();
    println!("[{}] empty={}", c, c.is_empty());

    let censored: String = "badword in a badword sentence".split(' ').map(|w| if w == "badword" { "*".repeat(w.len()) } else { w.to_string() }).collect::<Vec<_>>().join(" ");
    println!("{}", censored);
    let mut s2 = String::from("abcdef");
    s2.replace_range(1..4, "XYZW");
    println!("{}", s2);
    let drained: String = s2.drain(..3).collect();
    println!("{} {}", drained, s2);
    let sorted_chars: String = {
        let mut v: Vec<char> = "rustacean".chars().collect();
        v.sort_unstable();
        v.dedup();
        v.into_iter().collect()
    };
    println!("{}", sorted_chars);
    println!("{:?}", "Hello".char_indices().filter(|(_, c)| c.is_lowercase()).map(|(i, _)| i).collect::<Vec<_>>());
}
