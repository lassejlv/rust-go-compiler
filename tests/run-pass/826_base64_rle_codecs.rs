const ALPHA: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

fn b64_encode(data: &[u8]) -> String {
    let mut out = String::with_capacity((data.len() + 2) / 3 * 4);
    for chunk in data.chunks(3) {
        let b = [chunk[0], *chunk.get(1).unwrap_or(&0), *chunk.get(2).unwrap_or(&0)];
        let n = (b[0] as u32) << 16 | (b[1] as u32) << 8 | b[2] as u32;
        for i in 0..4 {
            if i <= chunk.len() { out.push(ALPHA[(n >> (18 - 6 * i) & 63) as usize] as char); } else { out.push('='); }
        }
    }
    out
}

fn b64_decode(s: &str) -> Result<Vec<u8>, String> {
    let mut lut = [255u8; 256];
    for (i, &c) in ALPHA.iter().enumerate() { lut[c as usize] = i as u8; }
    let bytes: Vec<u8> = s.bytes().filter(|b| !b.is_ascii_whitespace()).collect();
    if bytes.len() % 4 != 0 { return Err(format!("bad length {}", bytes.len())); }
    let mut out = vec![];
    for (ci, q) in bytes.chunks(4).enumerate() {
        let pad = q.iter().rev().take_while(|&&c| c == b'=').count();
        let mut n = 0u32;
        for (i, &c) in q.iter().enumerate() {
            let v = if c == b'=' { if i < 4 - pad { return Err("misplaced pad".into()); } 0 } else { let v = lut[c as usize]; if v == 255 { return Err(format!("bad char {:?} in chunk {}", c as char, ci)); } v };
            n = n << 6 | v as u32;
        }
        let full = [(n >> 16) as u8, (n >> 8) as u8, n as u8];
        out.extend_from_slice(&full[..3 - pad]);
    }
    Ok(out)
}

fn rle_encode(s: &str) -> String {
    let mut out = String::new();
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        let mut n = 1;
        while chars.next_if_eq(&c).is_some() { n += 1; }
        if n > 1 { out += &n.to_string(); }
        out.push(c);
    }
    out
}

fn rle_decode(s: &str) -> String {
    let mut out = String::new();
    let mut num = 0usize;
    for c in s.chars() {
        if let Some(d) = c.to_digit(10) { num = num * 10 + d as usize; } else { out.extend(std::iter::repeat(c).take(num.max(1))); num = 0; }
    }
    out
}

fn crc32(data: &[u8]) -> u32 {
    let mut table = [0u32; 256];
    for i in 0..256u32 {
        let mut c = i;
        for _ in 0..8 { c = if c & 1 != 0 { 0xEDB88320 ^ (c >> 1) } else { c >> 1 }; }
        table[i as usize] = c;
    }
    !data.iter().fold(!0u32, |c, &b| table[((c ^ b as u32) & 0xff) as usize] ^ (c >> 8))
}

fn adler32(data: &[u8]) -> u32 {
    let (mut a, mut b) = (1u32, 0u32);
    for &x in data { a = (a + x as u32) % 65521; b = (b + a) % 65521; }
    b << 16 | a
}

fn hex(data: &[u8]) -> String { data.iter().map(|b| format!("{:02x}", b)).collect() }
fn unhex(s: &str) -> Option<Vec<u8>> {
    if s.len() % 2 != 0 { return None; }
    (0..s.len()).step_by(2).map(|i| u8::from_str_radix(s.get(i..i + 2)?, 16).ok()).collect()
}

fn xor_cipher(data: &[u8], key: &[u8]) -> Vec<u8> { data.iter().zip(key.iter().cycle()).map(|(d, k)| d ^ k).collect() }

fn main() {
    for s in ["", "f", "fo", "foo", "foob", "fooba", "foobar", "Many hands make light work.", "ünïcødé ✓"] {
        let e = b64_encode(s.as_bytes());
        let d = b64_decode(&e).unwrap();
        println!("{:?} -> {} -> ok={}", s, e, d == s.as_bytes());
    }
    for bad in ["abc", "ab$d", "a=bc", "TWFu\nTWFu"] { println!("decode {:?}: {:?}", bad, b64_decode(bad).map(|v| String::from_utf8_lossy(&v).into_owned())); }
    let bin: Vec<u8> = (0..=255u8).step_by(17).collect();
    println!("{} {}", b64_encode(&bin), hex(&bin));
    for s in ["WWWWWWWWWWWWBWWWWWWWWWWWWBBBWWWWWWWWWWWWWWWWWWWWWWWWBWWWWWWWWWWWWWW", "abc", "aabbbcccc", "", "zzzzzzzzzzzz€€"] {
        let e = rle_encode(s);
        println!("rle {:?} -> {:?} round={}", s.len(), e, rle_decode(&e) == s);
    }
    for s in ["", "a", "abc", "The quick brown fox jumps over the lazy dog", "123456789"] {
        println!("crc32({:?}) = {:08x} adler = {:08x}", s, crc32(s.as_bytes()), adler32(s.as_bytes()));
    }
    println!("{:?} {:?} {:?}", unhex("deadBEEF"), unhex("abc"), unhex("zz"));
    let enc = xor_cipher(b"attack at dawn", b"key");
    println!("{} {}", hex(&enc), String::from_utf8(xor_cipher(&enc, b"key")).unwrap());
    let mut freq = [0usize; 26];
    for c in "the quick brown fox jumps over the lazy dog".bytes().filter(u8::is_ascii_lowercase) { freq[(c - b'a') as usize] += 1; }
    let top: Vec<(char, usize)> = { let mut v: Vec<_> = freq.iter().enumerate().map(|(i, &n)| ((b'a' + i as u8) as char, n)).filter(|p| p.1 > 1).collect(); v.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0))); v };
    println!("{:?}", top);
}
