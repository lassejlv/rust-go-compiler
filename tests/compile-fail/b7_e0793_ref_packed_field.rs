//~ ERROR E0793
// A reference to an unaligned field of a packed struct.
#[repr(C, packed)]
struct Header {
    tag: u8,
    len: u32,
}
fn main() {
    let h = Header { tag: 1, len: 2 };
    let r = &h.len;
    println!("{} {}", h.tag, r);
}
