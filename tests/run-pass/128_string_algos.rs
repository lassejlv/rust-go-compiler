fn caesar(s: &str, shift: i32) -> String {
    s.chars()
        .map(|c| {
            if c.is_ascii_alphabetic() {
                let base = if c.is_ascii_lowercase() { b'a' } else { b'A' } as i32;
                (((c as i32 - base + shift).rem_euclid(26)) + base) as u8 as char
            } else {
                c
            }
        })
        .collect()
}

fn vigenere(s: &str, key: &str, decrypt: bool) -> String {
    let k: Vec<i32> = key.bytes().map(|b| (b.to_ascii_lowercase() - b'a') as i32).collect();
    let mut i = 0;
    s.chars()
        .map(|c| {
            if c.is_ascii_alphabetic() {
                let shift = if decrypt { -k[i % k.len()] } else { k[i % k.len()] };
                i += 1;
                caesar(&c.to_string(), shift).chars().next().unwrap()
            } else {
                c
            }
        })
        .collect()
}

fn is_palindrome(s: &str) -> bool {
    let cs: Vec<char> = s.chars().filter(|c| c.is_alphanumeric()).map(|c| c.to_ascii_lowercase()).collect();
    let n = cs.len();
    (0..n / 2).all(|i| cs[i] == cs[n - 1 - i])
}

fn longest_palindrome(s: &str) -> &str {
    let b = s.as_bytes();
    let n = b.len();
    let (mut best_lo, mut best_len) = (0, 0);
    for center in 0..2 * n {
        let mut lo = (center / 2) as isize;
        let mut hi = lo + (center % 2) as isize;
        while lo >= 0 && (hi as usize) < n && b[lo as usize] == b[hi as usize] {
            lo -= 1;
            hi += 1;
        }
        let len = (hi - lo - 1) as usize;
        if len > best_len {
            best_len = len;
            best_lo = (lo + 1) as usize;
        }
    }
    &s[best_lo..best_lo + best_len]
}

fn edit_distance(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let mut dp = vec![vec![0usize; b.len() + 1]; a.len() + 1];
    for i in 0..=a.len() {
        dp[i][0] = i;
    }
    for j in 0..=b.len() {
        dp[0][j] = j;
    }
    for i in 1..=a.len() {
        for j in 1..=b.len() {
            let cost = if a[i - 1] == b[j - 1] { 0 } else { 1 };
            dp[i][j] = (dp[i - 1][j] + 1).min(dp[i][j - 1] + 1).min(dp[i - 1][j - 1] + cost);
        }
    }
    dp[a.len()][b.len()]
}

fn kmp_search(text: &str, pat: &str) -> Vec<usize> {
    let t = text.as_bytes();
    let p = pat.as_bytes();
    let mut fail = vec![0usize; p.len()];
    let mut k = 0;
    for i in 1..p.len() {
        while k > 0 && p[i] != p[k] {
            k = fail[k - 1];
        }
        if p[i] == p[k] {
            k += 1;
        }
        fail[i] = k;
    }
    let mut res = Vec::new();
    k = 0;
    for (i, &c) in t.iter().enumerate() {
        while k > 0 && c != p[k] {
            k = fail[k - 1];
        }
        if c == p[k] {
            k += 1;
        }
        if k == p.len() {
            res.push(i + 1 - k);
            k = fail[k - 1];
        }
    }
    res
}

fn main() {
    let msg = "Hello, World! xyz";
    let enc = caesar(msg, 3);
    println!("caesar +3: {}", enc);
    println!("caesar -3: {}", caesar(&enc, -3));
    println!("rot13 twice: {}", caesar(&caesar(msg, 13), 13));
    let v = vigenere("Attack at dawn!", "LEMON", false);
    println!("vigenere: {}", v);
    println!("vigenere decrypt: {}", vigenere(&v, "LEMON", true));
    for s in ["Racecar", "Was it a car or a cat I saw?", "No lemon, no melon", "palindrome", "", "x"] {
        println!("{:?} -> {}", s, is_palindrome(s));
    }
    for s in ["babad", "forgeeksskeegfor", "abacdfgdcaba", "a", "abcd"] {
        println!("longest palindrome in {}: {}", s, longest_palindrome(s));
    }
    for (a, b) in [("kitten", "sitting"), ("flaw", "lawn"), ("", "abc"), ("rust", "rust"), ("intention", "execution")] {
        println!("edit_distance({}, {}) = {}", a, b, edit_distance(a, b));
    }
    println!("kmp: {:?}", kmp_search("abababcabababcab", "ababc"));
    println!("kmp overlapping: {:?}", kmp_search("aaaaa", "aa"));
    println!("kmp none: {:?}", kmp_search("hello", "xyz"));
}
