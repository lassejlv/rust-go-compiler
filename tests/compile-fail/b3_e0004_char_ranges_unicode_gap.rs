//~ ERROR E0004
fn f(c: char) -> bool {
    match c {
        '\0'..='\u{D7FF}' => true,
        '\u{E000}'..='\u{FFFF}' => false,
    }
}
fn main() { f('a'); }
