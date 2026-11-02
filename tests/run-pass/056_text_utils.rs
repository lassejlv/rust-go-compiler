fn title_case(s: &str) -> String {
    s.split_whitespace()
        .map(|w| {
            let mut chars = w.chars();
            match chars.next() {
                Some(c) => c.to_uppercase().chain(chars.flat_map(|c| c.to_lowercase())).collect(),
                None => String::new(),
            }
        })
        .collect::<Vec<String>>()
        .join(" ")
}

fn reverse_words(s: &str) -> String {
    s.split_whitespace().rev().collect::<Vec<_>>().join(" ")
}

fn rle_encode(s: &str) -> String {
    let mut out = String::new();
    let chars: Vec<char> = s.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        let mut j = i;
        while j < chars.len() && chars[j] == c {
            j += 1;
        }
        out.push_str(&(j - i).to_string());
        out.push(c);
        i = j;
    }
    out
}

fn rle_decode(s: &str) -> String {
    let mut out = String::new();
    let mut count = 0usize;
    for c in s.chars() {
        if let Some(d) = c.to_digit(10) {
            count = count * 10 + d as usize;
        } else {
            for _ in 0..count {
                out.push(c);
            }
            count = 0;
        }
    }
    out
}

fn count_vowels(s: &str) -> usize {
    s.chars().filter(|c| "aeiouAEIOU".contains(*c)).count()
}

fn is_anagram(a: &str, b: &str) -> bool {
    let mut x: Vec<char> = a.to_lowercase().chars().filter(|c| c.is_alphabetic()).collect();
    let mut y: Vec<char> = b.to_lowercase().chars().filter(|c| c.is_alphabetic()).collect();
    x.sort();
    y.sort();
    x == y
}

fn wrap(text: &str, width: usize) -> Vec<String> {
    let mut lines = Vec::new();
    let mut cur = String::new();
    for word in text.split_whitespace() {
        if !cur.is_empty() && cur.len() + 1 + word.len() > width {
            lines.push(cur);
            cur = String::new();
        }
        if !cur.is_empty() {
            cur.push(' ');
        }
        cur.push_str(word);
    }
    if !cur.is_empty() {
        lines.push(cur);
    }
    lines
}

fn snake_to_camel(s: &str) -> String {
    let mut out = String::new();
    let mut upper = false;
    for c in s.chars() {
        if c == '_' {
            upper = true;
        } else if upper {
            out.push(c.to_ascii_uppercase());
            upper = false;
        } else {
            out.push(c);
        }
    }
    out
}

fn main() {
    println!("{}", title_case("the qUICK brown fOX"));
    println!("{}", reverse_words("one two three four"));
    let enc = rle_encode("aaabccddddde");
    println!("rle: {} -> {}", enc, rle_decode(&enc));
    println!("rle long: {}", rle_encode(&"z".repeat(12)));
    println!("vowels: {}", count_vowels("Programming Language Theory"));
    println!("anagram: {} {}", is_anagram("Dormitory", "dirty room"), is_anagram("hello", "world"));
    let text = "Rust is a multi-paradigm, general-purpose programming language that emphasizes performance, type safety, and concurrency.";
    for (i, line) in wrap(text, 30).iter().enumerate() {
        println!("{:2}| {:<30} |", i + 1, line);
    }
    println!("{}", snake_to_camel("parse_http_response_code"));
}
