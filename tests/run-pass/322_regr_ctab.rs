const fn build() -> [u32; 256] { let mut t = [0u32; 256]; let mut i = 0; while i < 256 { let mut c = i as u32; let mut k = 0; while k < 8 { c = if c & 1 != 0 { 0xEDB88320 ^ (c >> 1) } else { c >> 1 }; k += 1; } t[i] = c; i += 1; } t }
struct T<const N: usize>;
impl<const N: usize> T<N> { const TABLE: [u32; 256] = build(); }
fn crc(data: &[u8]) -> u32 { let mut c = !0u32; for &b in data { c = T::<1>::TABLE[((c ^ b as u32) & 0xff) as usize] ^ (c >> 8); } !c }
fn main() { let d = vec![7u8; 2_000_000]; println!("{:08x}", crc(&d)); }
