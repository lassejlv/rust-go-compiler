//~ ERROR E0004
fn class(b: u8) -> &'static str {
    match b {
        0..=31 => "ctrl",
        32..=126 => "print",
        128..=255 => "high",
    }
}
fn main() { class(1); }
