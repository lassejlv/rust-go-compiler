// Real hash/checksum algorithms implemented with wrapping arithmetic and bit
// rotation: SHA-256, MD5, CRC-32 (table built by const fn), Adler-32,
// FNV-1a 64, plus Base64 encode/decode and hex formatting.
const fn crc_table() -> [u32; 256] {
    let mut t = [0u32; 256];
    let mut i = 0;
    while i < 256 {
        let mut c = i as u32;
        let mut k = 0;
        while k < 8 {
            c = if c & 1 != 0 { 0xEDB88320 ^ (c >> 1) } else { c >> 1 };
            k += 1;
        }
        t[i] = c;
        i += 1;
    }
    t
}
static CRC: [u32; 256] = crc_table();

fn crc32(data: &[u8]) -> u32 {
    !data.iter().fold(!0u32, |c, &b| CRC[((c ^ b as u32) & 0xff) as usize] ^ (c >> 8))
}

fn adler32(data: &[u8]) -> u32 {
    let (mut a, mut b) = (1u32, 0u32);
    for &x in data { a = (a + x as u32) % 65521; b = (b + a) % 65521; }
    (b << 16) | a
}

fn fnv1a(data: &[u8]) -> u64 {
    data.iter().fold(0xcbf29ce484222325u64, |h, &b| (h ^ b as u64).wrapping_mul(0x100000001b3))
}

const K256: [u32; 64] = [
    0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5,
    0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174,
    0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
    0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7, 0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967,
    0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85,
    0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
    0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
    0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2,
];

fn pad(msg: &[u8], big_endian: bool) -> Vec<u8> {
    let mut m = msg.to_vec();
    let bits = (msg.len() as u64).wrapping_mul(8);
    m.push(0x80);
    while m.len() % 64 != 56 { m.push(0); }
    m.extend_from_slice(&if big_endian { bits.to_be_bytes() } else { bits.to_le_bytes() });
    m
}

fn sha256(msg: &[u8]) -> [u8; 32] {
    let mut h: [u32; 8] = [0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab, 0x5be0cd19];
    for block in pad(msg, true).chunks_exact(64) {
        let mut w = [0u32; 64];
        for (i, c) in block.chunks_exact(4).enumerate() { w[i] = u32::from_be_bytes(c.try_into().unwrap()); }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16].wrapping_add(s0).wrapping_add(w[i - 7]).wrapping_add(s1);
        }
        let [mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut hh] = h;
        for i in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ (!e & g);
            let t1 = hh.wrapping_add(s1).wrapping_add(ch).wrapping_add(K256[i]).wrapping_add(w[i]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let t2 = s0.wrapping_add(maj);
            hh = g; g = f; f = e; e = d.wrapping_add(t1); d = c; c = b; b = a; a = t1.wrapping_add(t2);
        }
        for (x, y) in h.iter_mut().zip([a, b, c, d, e, f, g, hh]) { *x = x.wrapping_add(y); }
    }
    let mut out = [0u8; 32];
    for (i, v) in h.iter().enumerate() { out[i * 4..i * 4 + 4].copy_from_slice(&v.to_be_bytes()); }
    out
}

fn md5(msg: &[u8]) -> [u8; 16] {
    let s: [u32; 64] = [7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22, 5, 9, 14, 20, 5, 9, 14, 20, 5, 9, 14, 20, 5, 9, 14, 20,
        4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23, 6, 10, 15, 21, 6, 10, 15, 21, 6, 10, 15, 21, 6, 10, 15, 21];
    let k: Vec<u32> = (0..64).map(|i| ((i as f64 + 1.0).sin().abs() * 4294967296.0) as u32).collect();
    let mut st = [0x67452301u32, 0xefcdab89, 0x98badcfe, 0x10325476];
    for block in pad(msg, false).chunks_exact(64) {
        let m: Vec<u32> = block.chunks_exact(4).map(|c| u32::from_le_bytes([c[0], c[1], c[2], c[3]])).collect();
        let [mut a, mut b, mut c, mut d] = st;
        for i in 0..64 {
            let (f, g) = match i / 16 {
                0 => ((b & c) | (!b & d), i),
                1 => ((d & b) | (!d & c), (5 * i + 1) % 16),
                2 => (b ^ c ^ d, (3 * i + 5) % 16),
                _ => (c ^ (b | !d), (7 * i) % 16),
            };
            let f2 = f.wrapping_add(a).wrapping_add(k[i]).wrapping_add(m[g]);
            a = d; d = c; c = b;
            b = b.wrapping_add(f2.rotate_left(s[i]));
        }
        st[0] = st[0].wrapping_add(a); st[1] = st[1].wrapping_add(b); st[2] = st[2].wrapping_add(c); st[3] = st[3].wrapping_add(d);
    }
    let mut out = [0u8; 16];
    for (i, v) in st.iter().enumerate() { out[i * 4..i * 4 + 4].copy_from_slice(&v.to_le_bytes()); }
    out
}

const B64: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
fn b64_encode(data: &[u8]) -> String {
    let mut out = String::new();
    for chunk in data.chunks(3) {
        let n = chunk.iter().enumerate().fold(0u32, |acc, (i, &b)| acc | (b as u32) << (16 - 8 * i));
        for i in 0..4 {
            if i <= chunk.len() { out.push(B64[((n >> (18 - 6 * i)) & 63) as usize] as char); } else { out.push('='); }
        }
    }
    out
}
fn b64_decode(s: &str) -> Result<Vec<u8>, String> {
    let mut out = Vec::new();
    let bytes: Vec<u8> = s.bytes().filter(|&b| b != b'=').collect();
    for chunk in bytes.chunks(4) {
        let mut n = 0u32;
        for (i, &c) in chunk.iter().enumerate() {
            let v = B64.iter().position(|&x| x == c).ok_or_else(|| format!("invalid char {:?}", c as char))?;
            n |= (v as u32) << (18 - 6 * i);
        }
        for i in 0..chunk.len().saturating_sub(1) { out.push((n >> (16 - 8 * i)) as u8); }
    }
    Ok(out)
}

fn hex(b: &[u8]) -> String { b.iter().map(|x| format!("{:02x}", x)).collect() }

fn main() {
    let inputs: [&[u8]; 5] = [b"", b"abc", b"The quick brown fox jumps over the lazy dog", b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq", &[0u8; 200]];
    for inp in inputs {
        let shown = if inp.len() > 20 { format!("{}...({} bytes)", String::from_utf8_lossy(&inp[..12]).escape_debug(), inp.len()) } else { format!("{:?}", String::from_utf8_lossy(inp)) };
        println!("{}", shown);
        println!("  sha256 {}", hex(&sha256(inp)));
        println!("  md5    {}", hex(&md5(inp)));
        println!("  crc32 {:08x} adler32 {:08x} fnv1a {:016x}", crc32(inp), adler32(inp), fnv1a(inp));
        let enc = b64_encode(inp);
        let dec = b64_decode(&enc).unwrap();
        println!("  b64 {} roundtrip={}", if enc.len() > 40 { format!("{}..", &enc[..40]) } else { enc.clone() }, dec == inp);
    }
    println!("crc table [1]={:08x} [255]={:08x}", CRC[1], CRC[255]);
    println!("b64 decode error: {:?}", b64_decode("ab$d"));
    println!("b64 of 1..5 bytes: {:?}", (1..=5).map(|n| b64_encode(&b"xyzzy"[..n])).collect::<Vec<_>>());
    let million_a: Vec<u8> = vec![b'a'; 100_000];
    println!("sha256(100k 'a') {}", hex(&sha256(&million_a)));
}
