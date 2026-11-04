//~ ERROR E0277
// Only the last field of a struct may be dynamically sized.
struct Packet {
    payload: [u8],
    len: usize,
}
fn main() {}
