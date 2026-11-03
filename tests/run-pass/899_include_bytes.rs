// include_bytes! yields a `&'static [u8; N]` (this file's own bytes).
struct AlignAs<B: ?Sized> {
    _align: [u32; 0],
    bytes: B,
}

static ALIGNED: &AlignAs<[u8]> = &AlignAs { _align: [], bytes: *include_bytes!("899_include_bytes.rs") };
static RAW: &[u8] = include_bytes!("899_include_bytes.rs");

fn main() {
    let b: &'static [u8; 12] = &[0; 12];
    let _ = b;
    let all = include_bytes!("899_include_bytes.rs");
    let arr = *include_bytes!("899_include_bytes.rs");
    println!("{}", all.len() == arr.len());
    println!("{}", RAW.len() == ALIGNED.bytes.len());
    println!("{:?}", &all[..15]);
    println!("{}", std::str::from_utf8(&ALIGNED.bytes[3..15]).unwrap());
    println!("{}", arr.iter().filter(|&&c| c == b'\n').count() > 10);
}
