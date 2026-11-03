//~ ERROR E0502
fn main() {
    let mut s = String::from("ab");
    let view = s.as_str();
    s.push_str(view);
}
