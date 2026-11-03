//~ ERROR E0502
fn main() {
    let mut s = String::from("abc");
    for c in s.chars() {
        if c == 'a' { s.push('x'); }
    }
}
